// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::action_controller::{Action, ActionSender};
use super::PendingAcknowledgement;
use crate::client::replies::reply_storage::CombinedReplyStorage;
use crate::client::{
    inbound_messages::{InputMessage, InputMessageReceiver},
    real_messages_control::real_traffic_stream::{BatchRealMessageSender, RealMessage},
    topology_control::TopologyAccessor,
};
use crypto::symmetric::stream_cipher;
use futures::StreamExt;
use log::*;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::forwarding::packet::MixPacket;
use nymsphinx::params::{PacketEncryptionAlgorithm, ReplySurbEncryptionAlgorithm};
use nymsphinx::preparer::MessagePreparer;
use nymsphinx::{acknowledgements::AckKey, addressing::clients::Recipient};
use rand::{CryptoRng, Rng};
use std::sync::Arc;

// #[cfg(feature = "reply-surb")]
// use crate::client::reply_key_storage::ReplyKeyStorage;

/// Module responsible for dealing with the received messages: splitting them, creating acknowledgements,
/// putting everything into sphinx packets, etc.
/// It also makes an initial sending attempt for said messages.
pub(super) struct InputMessageListener<R>
where
    R: CryptoRng + Rng,
{
    ack_key: Arc<AckKey>,
    ack_recipient: Recipient,
    input_receiver: InputMessageReceiver,
    message_preparer: MessagePreparer<R>,
    action_sender: ActionSender,
    real_message_sender: BatchRealMessageSender,
    topology_access: TopologyAccessor,
    // #[cfg(feature = "reply-surb")]
    // reply_key_storage: ReplyKeyStorage,
    reply_storage: CombinedReplyStorage,
}

pub(super) struct Config {
    max_per_sender_buffer_size: usize,
}

impl<R> InputMessageListener<R>
where
    R: CryptoRng + Rng,
{
    // at this point I'm not entirely sure how to deal with this warning without
    // some considerable refactoring
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        ack_key: Arc<AckKey>,
        ack_recipient: Recipient,
        input_receiver: InputMessageReceiver,
        message_preparer: MessagePreparer<R>,
        action_sender: ActionSender,
        real_message_sender: BatchRealMessageSender,
        topology_access: TopologyAccessor,
        reply_storage: CombinedReplyStorage,
        // #[cfg(feature = "reply-surb")] reply_key_storage: ReplyKeyStorage,
    ) -> Self {
        InputMessageListener {
            ack_key,
            ack_recipient,
            input_receiver,
            message_preparer,
            action_sender,
            real_message_sender,
            topology_access,
            // #[cfg(feature = "reply-surb")]
            // reply_key_storage,
            reply_storage,
        }
    }

    async fn handle_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
    ) -> Option<Vec<RealMessage>> {
        if !self.reply_storage.contains_surbs_for(&recipient_tag) {
            warn!("received reply request for {:?} but we don't have any surbs stored for that recipient!", recipient_tag);
            return None;
        }

        // TODO: lower to debug/trace
        info!("handling reply to {:?}", recipient_tag);
        let fragments = self.message_preparer.prepare_and_split_reply(data);

        info!("This reply requires {:?} SURBs", fragments.len());

        let (surbs, surbs_left) = self
            .reply_storage
            .get_reply_surbs(&recipient_tag, fragments.len());

        if let Some(reply_surbs) = surbs {
            // TODO: simplify, tidy up and move elsewhere
            let topology_permit = self.topology_access.get_read_permit().await;
            let topology =
                match topology_permit.try_get_valid_topology_ref(&self.ack_recipient, None) {
                    Some(topology_ref) => topology_ref,
                    None => {
                        warn!("Could not process the message - the network topology is invalid");
                        return None;
                    }
                };

            let mut packets = Vec::with_capacity(reply_surbs.len());
            for (fragment, reply_surb) in fragments.into_iter().zip(reply_surbs.into_iter()) {
                // TODO: this should be happening inside message_preparer!!!
                let fragment_id = fragment.fragment_identifier();
                let (ack_delay, surb_ack_bytes) = self
                    .message_preparer
                    .generate_surb_ack(fragment_id, topology, &self.ack_key)
                    .expect("TODO: handle this error")
                    .prepare_for_sending();

                let mut fragment_data = fragment.into_bytes();

                stream_cipher::encrypt_in_place::<ReplySurbEncryptionAlgorithm>(
                    reply_surb.encryption_key().inner(),
                    &stream_cipher::zero_iv::<ReplySurbEncryptionAlgorithm>(),
                    &mut fragment_data,
                );

                // TODO: extract it to different method
                // combine it together as follows:
                // SURB_ACK_FIRST_HOP || SURB_ACK_DATA || KEY_DIGEST || E (REPLY_MESSAGE || 1 || 0*)
                // (note: surb_ack_bytes contains SURB_ACK_FIRST_HOP || SURB_ACK_DATA )
                let packet_payload: Vec<_> = surb_ack_bytes
                    .into_iter()
                    .chain(reply_surb.encryption_key().compute_digest().iter().copied())
                    .chain(fragment_data.into_iter())
                    .collect();

                // the unwrap here is fine as the failures can only originate from attempting to use invalid payload lenghts
                // and we just very carefully constructed a (presumably) valid one
                let (sphinx_packet, first_hop) = reply_surb
                    .apply_surb(&packet_payload, Some(self.message_preparer.packet_size))
                    .unwrap();

                let mix_packet = MixPacket::new(first_hop, sphinx_packet, Default::default());
                let real_message = RealMessage::new(mix_packet, fragment_id);
                packets.push(real_message);
            }

            Some(packets)
        } else {
            // TODO: here be the logic for surb requests and I guess delegation to the surbs handler
            panic!(
                "we don't have enough surbs : (  we only have {} left",
                surbs_left
            )
        }
    }

    // we require topology for replies to generate surb_acks
    async fn handle_reply_with_surb(
        &mut self,
        reply_surb: ReplySurb,
        data: Vec<u8>,
    ) -> Option<RealMessage> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match topology_permit.try_get_valid_topology_ref(&self.ack_recipient, None) {
            Some(topology_ref) => topology_ref,
            None => {
                warn!("Could not process the message - the network topology is invalid");
                return None;
            }
        };

        match self
            .message_preparer
            .prepare_reply_for_use(data, reply_surb, topology, &self.ack_key)
            .await
        {
            Ok((mix_packet, reply_id)) => {
                // TODO: later probably write pending ack here
                // and deal with them....
                // ... somehow
                Some(RealMessage::new(mix_packet, reply_id))
            }
            Err(err) => {
                // TODO: should we have some mechanism to indicate to the user that the `reply_surb`
                // could be reused since technically it wasn't used up here?
                warn!("failed to deal with received reply surb - {:?}", err);
                None
            }
        }
    }

    // async fn split_raw_message_into_fragments(
    //     &mut self,
    //     raw_message: Vec<u8>,
    //     reply_surbs: u32,
    // ) -> Option<()> {
    //     let topology_permit = self.topology_access.get_read_permit().await;
    //     let topology = match topology_permit
    //         .try_get_valid_topology_ref(&self.ack_recipient, Some(&recipient))
    //     {
    //         Some(topology_ref) => topology_ref,
    //         None => {
    //             warn!("Could not process the message - the network topology is invalid");
    //             return None;
    //         }
    //     };
    //
    //     // split the message, attach optional reply surb
    //     let (split_message, reply_keys) = self
    //         .message_preparer
    //         .prepare_and_split_message(content, reply_surbs, topology)
    //         .expect("somehow the topology was invalid after all!");
    // }

    async fn handle_fresh_message(
        &mut self,
        recipient: Recipient,
        content: Vec<u8>,
        reply_surbs: u32,
    ) -> Option<Vec<RealMessage>> {
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = match topology_permit
            .try_get_valid_topology_ref(&self.ack_recipient, Some(&recipient))
        {
            Some(topology_ref) => topology_ref,
            None => {
                warn!("Could not process the message - the network topology is invalid");
                return None;
            }
        };

        // split the message, attach optional reply surb
        let (split_message, reply_keys) = self
            .message_preparer
            .prepare_and_split_message(content, reply_surbs, topology)
            .expect("somehow the topology was invalid after all!");

        log::error!("here we need to store {} reply keys", reply_keys.len());
        self.reply_storage.insert_multiple_surb_keys(reply_keys);

        // todo!("handle reply keys: either have storage on THIS struct or move it with a channel or something");

        // #[cfg(feature = "reply-surb")]
        // if let Some(reply_key) = reply_key {
        //     self.reply_key_storage
        //         .insert_encryption_key(reply_key)
        //         .expect("Failed to insert surb reply key to the store!")
        // }
        //
        // #[cfg(not(feature = "reply-surb"))]
        // let _reply_key = reply_key;

        // encrypt chunks, put them inside sphinx packets and generate acks
        let mut pending_acks = Vec::with_capacity(split_message.len());
        let mut real_messages = Vec::with_capacity(split_message.len());
        for message_chunk in split_message {
            // we need to clone it because we need to keep it in memory in case we had to retransmit
            // it. And then we'd need to recreate entire ACK again.
            let chunk_clone = message_chunk.clone();
            let prepared_fragment = self
                .message_preparer
                .prepare_chunk_for_sending(chunk_clone, topology, &self.ack_key, &recipient)
                .unwrap();

            real_messages.push(RealMessage::new(
                prepared_fragment.mix_packet,
                message_chunk.fragment_identifier(),
            ));

            pending_acks.push(PendingAcknowledgement::new(
                message_chunk,
                prepared_fragment.total_delay,
                recipient,
            ));
        }

        // tells the controller to put this into the hashmap
        self.action_sender
            .unbounded_send(Action::new_insert(pending_acks))
            .unwrap();

        Some(real_messages)
    }

    async fn on_input_message(&mut self, msg: InputMessage) {
        let real_messages = match msg {
            InputMessage::Regular {
                recipient,
                data,
                reply_surbs,
            } => {
                self.handle_fresh_message(recipient, data, reply_surbs)
                    .await
            }
            InputMessage::ReplyWithSurb { reply_surb, data } => self
                .handle_reply_with_surb(reply_surb, data)
                .await
                .map(|message| vec![message]),
            InputMessage::Reply {
                recipient_tag,
                data,
            } => self.handle_reply(recipient_tag, data).await,
        };

        // there's no point in trying to send nothing
        if let Some(real_messages) = real_messages {
            // tells real message sender (with the poisson timer) to send this to the mix network
            self.real_message_sender
                .unbounded_send(real_messages)
                .unwrap();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started InputMessageListener with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                input_msg = self.input_receiver.next() => match input_msg {
                    Some(input_msg) => {
                        self.on_input_message(input_msg).await;
                    },
                    None => {
                        log::trace!("InputMessageListener: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv() => {
                    log::trace!("InputMessageListener: Received shutdown");
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("InputMessageListener: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn run(&mut self) {
        debug!("Started InputMessageListener without graceful shutdown support");
        while let Some(input_msg) = self.input_receiver.next().await {
            self.on_input_message(input_msg).await;
        }
    }
}
