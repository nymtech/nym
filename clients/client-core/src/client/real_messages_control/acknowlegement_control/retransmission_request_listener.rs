// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{PendingAcksMap, RetransmissionRequestReceiver};
use crate::client::{
    real_messages_control::real_traffic_stream::{RealMessage, RealMessageSender},
    topology_control::TopologyAccessor,
};
use futures::StreamExt;
use log::*;
use nymsphinx::preparer::MessagePreparer;
use nymsphinx::{
    acknowledgements::AckKey, addressing::clients::Recipient,
    chunking::fragment::FragmentIdentifier,
};
use rand::{CryptoRng, Rng};
use std::sync::Arc;

// responsible for packet retransmission upon fired timer
pub(super) struct RetransmissionRequestListener<R>
where
    R: CryptoRng + Rng,
{
    ack_key: Arc<AckKey>,
    ack_recipient: Recipient,
    message_preparer: MessagePreparer<R>,
    pending_acks: PendingAcksMap,
    real_message_sender: RealMessageSender,
    request_receiver: RetransmissionRequestReceiver,
    topology_access: TopologyAccessor,
}

impl<R> RetransmissionRequestListener<R>
where
    R: CryptoRng + Rng,
{
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_recipient: Recipient,
        message_preparer: MessagePreparer<R>,
        pending_acks: PendingAcksMap,
        real_message_sender: RealMessageSender,
        request_receiver: RetransmissionRequestReceiver,
        topology_access: TopologyAccessor,
    ) -> Self {
        RetransmissionRequestListener {
            ack_key,
            ack_recipient,
            message_preparer,
            pending_acks,
            real_message_sender,
            request_receiver,
            topology_access,
        }
    }

    async fn on_retransmission_request(&mut self, frag_id: FragmentIdentifier) {
        let pending_acks_map_read_guard = self.pending_acks.read().await;

        let unreceived_ack_fragment = match pending_acks_map_read_guard.get(&frag_id) {
            Some(pending_ack) => pending_ack,
            // this can actually happen when ack retransmission times out while `on_ack` is being processed
            // 1. `retransmission_sender.unbounded_send(frag_id).unwrap()` happens thus triggering this function
            // 2. at the same time ack is received and fully processed (which takes pending_acks *WRITE* lock!!) -> ack is removed from the map + `self.pending_acks.read()` blocks
            // 3. `on_retransmission_request` manages to get read lock, but the entry was already removed
            None => {
                info!("wanted to retransmit ack'd fragment");
                return;
            }
        };

        let packet_recipient = unreceived_ack_fragment.recipient.clone();
        let chunk_clone = unreceived_ack_fragment.message_chunk.clone();
        let frag_id = unreceived_ack_fragment.message_chunk.fragment_identifier();

        // TODO: we need some proper benchmarking here to determine whether it could
        // be more efficient to just get write lock and keep it while doing sphinx computation,
        // but my gut feeling tells me we should re-acquire it.
        drop(pending_acks_map_read_guard);

        let topology_permit = self.topology_access.get_read_permit().await;
        let topology_ref_option = topology_permit
            .try_get_valid_topology_ref(&self.ack_recipient, Some(&packet_recipient));
        if topology_ref_option.is_none() {
            warn!("Could not retransmit the packet - the network topology is invalid");
            // TODO: perhaps put back into pending acks and reset the timer?
            return;
        }
        let topology_ref = topology_ref_option.unwrap();

        let prepared_fragment = self
            .message_preparer
            .prepare_chunk_for_sending(chunk_clone, topology_ref, &self.ack_key, &packet_recipient)
            .unwrap();

        // minor optimization to not hold the permit while we no longer need it and might have to block
        // waiting for the write lock on `pending_acks`
        drop(topology_permit);

        // for this to actually return a None, the following sequence of events needs to happen:
        // 0. recall that up until this point we're holding a READ lock, so nobody else can WRITE
        // 1. `on_retransmission_request` is called - processing takes a while (we need to create SPHINX packet, etc.)
        // 2. at the same time we receive DELAYED (i.e. post timeout) ack for the packet we are about to retransmit
        // 3. the procedure to remove the pending ack waits for the WRITE lock and acquires it in the tiny window
        // between when READ lock is dropped and WRITE lock is reacquired in this method
        // 4. the pending ack is removed and when WRITE lock is acquired here, `None` is returned

        // TODO: benchmark whether it wouldn't be potentially more efficient to acquire WRITE lock at the very beginning of the method
        // one major drawback: nobody else could READ while we're preparing two sphinx packets, encrypting data, etc.
        if let Some(pending_ack) = self.pending_acks.write().await.get_mut(&frag_id) {
            pending_ack.update_delay(prepared_fragment.total_delay);

            self.real_message_sender
                .unbounded_send(RealMessage::new(
                    prepared_fragment.first_hop_address,
                    prepared_fragment.sphinx_packet,
                    frag_id,
                ))
                .unwrap();
        } else {
            // later on we will want this to be decreased to 'debug' (or maybe not?)
            info!("received an ack after timeout, but before retransmission went through")
        }
    }

    pub(super) async fn run(&mut self) {
        debug!("Started RetransmissionRequestListener");
        while let Some(frag_id) = self.request_receiver.next().await {
            self.on_retransmission_request(frag_id).await;
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}
