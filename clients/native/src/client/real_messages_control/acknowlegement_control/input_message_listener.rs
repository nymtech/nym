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

use super::{PendingAcknowledgement, PendingAcksMap};
use crate::client::{
    inbound_messages::{InputMessage, InputMessageReceiver},
    real_messages_control::real_traffic_stream::{RealMessage, RealMessageSender},
    topology_control::TopologyAccessor,
};
use futures::StreamExt;
use log::*;
use nymsphinx::{
    acknowledgements::AckAes128Key, addressing::clients::Recipient, chunking::MessageChunker,
};
use rand::{CryptoRng, Rng};
use std::sync::Arc;
use topology::NymTopology;

// responsible for splitting received message and initial sending attempt
// possible future TODO: the fields are IDENTICAL to the ones in RetransmissionRequestListener
// perhaps some code could be shared?
pub(super) struct InputMessageListener<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    ack_key: Arc<AckAes128Key>,
    ack_recipient: Recipient,
    input_receiver: InputMessageReceiver,
    message_chunker: MessageChunker<R>,
    pending_acks: PendingAcksMap,
    real_message_sender: RealMessageSender,
    topology_access: TopologyAccessor<T>,
}

impl<R, T> InputMessageListener<R, T>
where
    R: CryptoRng + Rng,
    T: NymTopology,
{
    pub(super) fn new(
        ack_key: Arc<AckAes128Key>,
        ack_recipient: Recipient,
        input_receiver: InputMessageReceiver,
        message_chunker: MessageChunker<R>,
        pending_acks: PendingAcksMap,
        real_message_sender: RealMessageSender,
        topology_access: TopologyAccessor<T>,
    ) -> Self {
        InputMessageListener {
            ack_key,
            ack_recipient,
            input_receiver,
            message_chunker,
            pending_acks,
            real_message_sender,
            topology_access,
        }
    }

    async fn on_input_message(&mut self, msg: InputMessage) {
        let (recipient, content) = msg.destruct();
        let split_message = self.message_chunker.split_message(&content);
        let topology_permit = self.topology_access.get_read_permit().await;

        let topology_ref_option =
            topology_permit.try_get_valid_topology_ref(&self.ack_recipient, &recipient);
        if topology_ref_option.is_none() {
            warn!("Could not process the message - the network topology is invalid");
            return;
        }
        let topology_ref = topology_ref_option.unwrap();

        let mut pending_acks = Vec::with_capacity(split_message.len());
        let mut real_messages = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // since the paths can be constructed, this CAN'T fail, if it does, there's a bug somewhere
            let frag_id = message_chunk.fragment_identifier();
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = message_chunk.clone();
            let (total_delay, (first_hop, packet)) = self
                .message_chunker
                .prepare_chunk_for_sending(chunk_clone, topology_ref, &self.ack_key, &recipient)
                .unwrap();

            real_messages.push(RealMessage::new(first_hop, packet, frag_id));

            let pending_ack =
                PendingAcknowledgement::new(message_chunk, total_delay, recipient.clone());

            pending_acks.push((frag_id, pending_ack));
        }

        // first insert pending_acks only then request fragments to be sent, otherwise you might get
        // some very nasty (and time-consuming to figure out...) race condition.
        let mut pending_acks_map_write_guard = self.pending_acks.write().await;
        for (frag_id, pending_ack) in pending_acks.into_iter() {
            if let Some(_) = pending_acks_map_write_guard.insert(frag_id, pending_ack) {
                panic!("Tried to insert duplicate pending ack")
            }
        }

        for real_message in real_messages {
            self.real_message_sender
                .unbounded_send(real_message)
                .unwrap();
        }
    }

    pub(super) async fn run(&mut self) {
        debug!("Started InputMessageListener");
        while let Some(input_msg) = self.input_receiver.next().await {
            self.on_input_message(input_msg).await;
        }
        error!("TODO: error msg. Or maybe panic?")
    }
}
