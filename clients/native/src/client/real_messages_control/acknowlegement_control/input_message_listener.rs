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
use crate::client::reply_key_storage::ReplyKeyStorage;
use crate::client::{
    inbound_messages::{InputMessage, InputMessageReceiver},
    real_messages_control::real_traffic_stream::{RealMessage, RealMessageSender},
    topology_control::TopologyAccessor,
};
use futures::StreamExt;
use log::*;
use nymsphinx::anonymous_replies::ReplySURB;
use nymsphinx::preparer::MessagePreparer;
use nymsphinx::{acknowledgements::AckKey, addressing::clients::Recipient};
use rand::{CryptoRng, Rng};
use std::sync::Arc;

// responsible for splitting received message and initial sending attempt
pub(super) struct InputMessageListener<R>
where
    R: CryptoRng + Rng,
{
    ack_key: Arc<AckKey>,
    ack_recipient: Recipient,
    input_receiver: InputMessageReceiver,
    message_preparer: MessagePreparer<R>,
    pending_acks: PendingAcksMap,
    real_message_sender: RealMessageSender,
    topology_access: TopologyAccessor,
    reply_key_storage: ReplyKeyStorage,
}

impl<R> InputMessageListener<R>
where
    R: CryptoRng + Rng,
{
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_recipient: Recipient,
        input_receiver: InputMessageReceiver,
        message_preparer: MessagePreparer<R>,
        pending_acks: PendingAcksMap,
        real_message_sender: RealMessageSender,
        topology_access: TopologyAccessor,
        reply_key_storage: ReplyKeyStorage,
    ) -> Self {
        InputMessageListener {
            ack_key,
            ack_recipient,
            input_receiver,
            message_preparer,
            pending_acks,
            real_message_sender,
            topology_access,
            reply_key_storage,
        }
    }

    async fn handle_reply(&mut self, reply_surb: ReplySURB, data: Vec<u8>) -> Option<RealMessage> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology_ref_option =
            topology_permit.try_get_valid_topology_ref(&self.ack_recipient, None);
        if topology_ref_option.is_none() {
            warn!("Could not process the message - the network topology is invalid");
            return None;
        }
        let topology = topology_ref_option.unwrap();

        match self
            .message_preparer
            .prepare_reply_for_use(data, reply_surb, topology, &self.ack_key)
        {
            Ok((reply_id, sphinx_packet, first_hop)) => {
                // TODO: later probably write pending ack here
                // and deal with them....
                // ... somehow

                Some(RealMessage::new(first_hop, sphinx_packet, reply_id))
            }
            Err(err) => {
                // TODO: should we have some mechanism to indicate to the user that the `reply_surb`
                // could be reused since technically it wasn't used up here?
                warn!("failed to deal with received reply surb - {:?}", err);
                None
            }
        }
    }

    async fn handle_fresh_message(
        &mut self,
        recipient: Recipient,
        content: Vec<u8>,
        with_reply_surb: bool,
    ) -> Vec<RealMessage> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology_ref_option =
            topology_permit.try_get_valid_topology_ref(&self.ack_recipient, Some(&recipient));
        if topology_ref_option.is_none() {
            warn!("Could not process the message - the network topology is invalid");
            return Vec::new();
        }
        let topology = topology_ref_option.unwrap();

        // split the message, attach optional reply surb
        let (split_message, reply_key) = self
            .message_preparer
            .prepare_and_split_message(content, with_reply_surb, topology)
            .expect("somehow the topology was invalid after all!");

        if let Some(reply_key) = reply_key {
            self.reply_key_storage
                .insert_encryption_key(reply_key)
                .expect("Failed to insert surb reply key to the store!")
        }

        // encrypt chunks, put them inside sphinx packets and generate acks
        let mut pending_acks = Vec::with_capacity(split_message.len());
        let mut real_messages = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // since the paths can be constructed, this CAN'T fail, if it does, there's a bug somewhere
            let frag_id = message_chunk.fragment_identifier();
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = message_chunk.clone();
            let prepared_fragment = self
                .message_preparer
                .prepare_chunk_for_sending(chunk_clone, topology, &self.ack_key, &recipient)
                .unwrap();

            real_messages.push(RealMessage::new(
                prepared_fragment.first_hop_address,
                prepared_fragment.sphinx_packet,
                frag_id,
            ));

            let pending_ack = PendingAcknowledgement::new(
                message_chunk,
                prepared_fragment.total_delay,
                recipient.clone(),
            );

            pending_acks.push((frag_id, pending_ack));
        }

        // first insert pending_acks only then request fragments to be sent, otherwise you might get
        // some very nasty (and time-consuming to figure out...) race condition.
        let mut pending_acks_map_write_guard = self.pending_acks.write().await;
        for (frag_id, pending_ack) in pending_acks.into_iter() {
            if pending_acks_map_write_guard
                .insert(frag_id, pending_ack)
                .is_some()
            {
                panic!("Tried to insert duplicate pending ack")
            }
        }

        real_messages
    }

    async fn on_input_message(&mut self, msg: InputMessage) {
        let real_messages = match msg {
            InputMessage::Fresh {
                recipient,
                data,
                with_reply_surb,
            } => {
                self.handle_fresh_message(recipient, data, with_reply_surb)
                    .await
            }
            InputMessage::Reply { reply_surb, data } => {
                if let Some(real_message) = self.handle_reply(reply_surb, data).await {
                    vec![real_message]
                } else {
                    return;
                }
            }
        };

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
