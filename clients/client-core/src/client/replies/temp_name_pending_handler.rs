// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::inbound_messages::InputMessageReceiver;
use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use crate::client::real_messages_control::real_traffic_stream::{
    BatchRealMessageSender, RealMessage,
};
use crate::client::replies::reply_storage::CombinedReplyStorage;
use crate::client::topology_control::{TopologyAccessor, TopologyReadPermit};
use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedSender;
use futures::StreamExt;
use log::{debug, info, warn};
use nymsphinx::acknowledgements::surb_ack::SurbAck;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::{AnonymousSenderTag, ReplyMessage};
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::chunking::fragment::Fragment;
use nymsphinx::params::PacketSize;
use nymsphinx::preparer::MessagePreparer;
use rand::{CryptoRng, Rng};
use std::cmp::min;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use topology::NymTopology;

// TODO: rename

#[derive(Debug, Clone)]
pub struct ToBeNamedSender(mpsc::UnboundedSender<ToBeNamedMessage>);

impl From<mpsc::UnboundedSender<ToBeNamedMessage>> for ToBeNamedSender {
    fn from(inner: mpsc::UnboundedSender<ToBeNamedMessage>) -> Self {
        ToBeNamedSender(inner)
    }
}

impl ToBeNamedSender {
    pub(crate) fn send_reply(&self, recipient: AnonymousSenderTag, message: Vec<u8>) {
        self.0
            .unbounded_send(ToBeNamedMessage::SendReply { recipient, message })
            .expect("ToBeNamedReceiver has died!")
    }

    pub(crate) fn send_additional_surbs(
        &self,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
    ) {
        self.0
            .unbounded_send(ToBeNamedMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
            })
            .expect("ToBeNamedReceiver has died!")
    }

    pub(crate) fn send_additional_surbs_request(&self, recipient: Recipient, amount: u32) {
        self.0
            .unbounded_send(ToBeNamedMessage::AdditionalSurbsRequest { recipient, amount })
            .expect("ToBeNamedReceiver has died!")
    }
}

pub type ToBeNamedReceiver = mpsc::UnboundedReceiver<ToBeNamedMessage>;

#[derive(Debug)]
pub enum ToBeNamedMessage {
    SendReply {
        recipient: AnonymousSenderTag,
        message: Vec<u8>,
    },

    AdditionalSurbs {
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
    },

    // Should this also be handled in here? it's technically a completely different side of the pipe
    // let's see how it works when combined, might split it before creating PR
    AdditionalSurbsRequest {
        recipient: Recipient,
        amount: u32,
    },
}

// the purpose of this task:
// - buffers split messages from input message listener if there were insufficient surbs to send them
// - upon getting extra surbs, resends them
// - so I guess it will handle all 'RepliableMessage' and requests from 'ReplyMessage'
// - replies to "give additional surbs" requests
// - will reply to future heartbeats

pub struct ToBeNamedPendingReplyController<R> {
    request_receiver: ToBeNamedReceiver,

    // expected_reliability: f32,
    // packet_size_used: PacketSize,
    pending_replies: HashMap<AnonymousSenderTag, VecDeque<Fragment>>,

    // I don't like the fact this exact set of fields exist on `InputMessageListener`, `RetransmissionRequestListener`
    // and THIS struct. it should probably be refactored into some shared structure
    // it's a huge mess of channels in here with repeated functionalities...
    ack_key: Arc<AckKey>,
    self_address: Recipient,
    message_preparer: MessagePreparer<R>,
    // action_sender: ActionSender,
    real_message_sender: BatchRealMessageSender,
    topology_access: TopologyAccessor,

    // TODO: it doesn't really need access to the keys, surbs themselves are enough
    reply_storage: CombinedReplyStorage,
}

impl<R> ToBeNamedPendingReplyController<R>
where
    R: CryptoRng + Rng,
{
    // TODO: don't make it public
    pub(crate) fn new(
        ack_key: Arc<AckKey>,
        ack_recipient: Recipient,
        message_preparer: MessagePreparer<R>,
        // action_sender: ActionSender,
        real_message_sender: BatchRealMessageSender,
        topology_access: TopologyAccessor,
        reply_storage: CombinedReplyStorage,
        request_receiver: ToBeNamedReceiver,
        // ) -> (Self, ToBeNamedSender) {
    ) -> Self {
        // let (request_sender, request_receiver) = mpsc::unbounded();

        // (
        ToBeNamedPendingReplyController {
            request_receiver,
            pending_replies: Default::default(),
            ack_key,
            self_address: ack_recipient,
            // action_sender,
            message_preparer,
            real_message_sender,
            topology_access,
            reply_storage,
        }
        //     ToBeNamedSender(request_sender),
        // )
    }

    // TODO: deal with code duplication later

    fn get_topology<'a>(&self, permit: &'a TopologyReadPermit<'a>) -> Option<&'a NymTopology> {
        match permit.try_get_valid_topology_ref(&self.self_address, None) {
            Some(topology_ref) => Some(topology_ref),
            None => {
                warn!("Could not process the message - the network topology is invalid");
                None
            }
        }
    }

    fn insert_pending_replies(&mut self, recipient: &AnonymousSenderTag, fragments: Vec<Fragment>) {
        if let Some(existing) = self.pending_replies.get_mut(recipient) {
            existing.append(&mut fragments.into())
        } else {
            self.pending_replies.insert(*recipient, fragments.into());
        }
    }

    async fn handle_send_reply(&mut self, recipient_tag: AnonymousSenderTag, data: Vec<u8>) {
        if !self.reply_storage.contains_surbs_for(&recipient_tag) {
            warn!("received reply request for {:?} but we don't have any surbs stored for that recipient!", recipient_tag);
            return;
        }

        // TODO: lower to debug/trace
        info!("handling reply to {:?}", recipient_tag);
        let fragments = self
            .message_preparer
            .prepare_and_split_reply(ReplyMessage::new_data_message(data));

        let required_surbs = fragments.len();
        info!("This reply requires {:?} SURBs", fragments.len());

        let (surbs, surbs_left) = self
            .reply_storage
            .get_reply_surbs(&recipient_tag, fragments.len());

        if let Some(reply_surbs) = surbs {
            // TODO: simplify, tidy up and move elsewhere
            let topology_permit = self.topology_access.get_read_permit().await;
            let topology = match self.get_topology(&topology_permit) {
                Some(topology) => topology,
                None => {
                    // without valid topology we can't do anything - put what we just retrieved back
                    drop(topology_permit);
                    self.reply_storage.insert_surbs(&recipient_tag, reply_surbs);
                    self.insert_pending_replies(&recipient_tag, fragments);
                    return;
                }
            };

            // TODO: shared code with so many different other parts lol
            let mut real_messages = Vec::with_capacity(reply_surbs.len());
            for (fragment, reply_surb) in fragments.into_iter().zip(reply_surbs.into_iter()) {
                // we need to clone it because we need to keep it in memory in case we had to retransmit
                // it. And then we'd need to recreate entire ACK again.
                let chunk_clone = fragment.clone();
                let prepared_fragment = self
                    .message_preparer
                    .prepare_reply_chunk_for_sending(
                        chunk_clone,
                        topology,
                        reply_surb,
                        &self.ack_key,
                    )
                    .unwrap();

                real_messages.push(RealMessage::new(
                    prepared_fragment.mix_packet,
                    fragment.fragment_identifier(),
                ));

                // TODO: deal with retransmission and acks here
            }

            // tells real message sender (with the poisson timer) to send this to the mix network
            self.real_message_sender
                .unbounded_send(real_messages)
                .unwrap();
        } else {
            self.insert_pending_replies(&recipient_tag, fragments);

            #[deprecated]
            //remove hardcoded 10
            self.request_additional_reply_surbs(&recipient_tag, 10 + required_surbs as u32)
                .await
                .expect("this temporary error handling HAS TO go")
        }
    }

    async fn request_additional_reply_surbs(
        &mut self,
        target: &AnonymousSenderTag,
        amount: u32,
    ) -> Option<()> {
        log::info!("requesting {} reply surbs ...", amount);

        let (reply_surb, _) = self
            .reply_storage
            .get_reply_surb_ignoring_threshold(target)?;
        let reply_surb = reply_surb?;
        let surbs_request = ReplyMessage::new_surb_request_message(self.self_address, amount);

        // TODO: this should really be more streamlined as we use the same pattern in multiple places
        let mut fragment = self.message_preparer.prepare_and_split_reply(surbs_request);
        assert_eq!(fragment.len(), 1, "our surbs request is tiny and should ALWAYS fit in a single sphinx packet, if it doesn't it means there's a serious issue somewhere and we should have blown up anyway");

        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit)?;

        let chunk = fragment.pop().unwrap();
        let chunk_clone = chunk.clone();
        let prepared_fragment = self
            .message_preparer
            .prepare_reply_chunk_for_sending(chunk_clone, topology, reply_surb, &self.ack_key)
            .unwrap();

        // TODO: ack and retransmission for the sucker...

        let real_messages =
            RealMessage::new(prepared_fragment.mix_packet, chunk.fragment_identifier());

        // tells real message sender (with the poisson timer) to send this to the mix network
        self.real_message_sender
            .unbounded_send(vec![real_messages])
            .unwrap();

        Some(())
    }

    fn pop_at_most_pending_replies(
        &mut self,
        from: &AnonymousSenderTag,
        amount: usize,
    ) -> Option<VecDeque<Fragment>> {
        // if possible, pop all pending replies, if not, pop only entries for which we'd have a reply surb
        let total = self.pending_replies.get(from)?.len();
        if total < amount {
            self.pending_replies.remove(from)
        } else {
            Some(
                self.pending_replies
                    .get_mut(from)?
                    .drain(..amount)
                    .collect(),
            )
        }
    }

    async fn try_clear_pending_queue(
        &mut self,
        from: &AnonymousSenderTag,
        available_surbs: &mut Vec<ReplySurb>,
    ) {
        println!("trying to clear pending queue");
        let surbs_left = available_surbs.len();

        println!("we have {} surbs on hand", surbs_left);

        if let Some(to_send) = self.pop_at_most_pending_replies(from, surbs_left) {
            println!("{} to clear", to_send.len());
            let topology_permit = self.topology_access.get_read_permit().await;
            let topology = match self.get_topology(&topology_permit) {
                Some(topology) => topology,
                None => {
                    // without valid topology we can't do anything - put what we just retrieved back
                    drop(topology_permit);
                    self.insert_pending_replies(from, to_send.into());
                    return;
                }
            };

            let mut real_messages = Vec::with_capacity(to_send.len());

            let elements = to_send.len();
            // we know `to_send.len() <= surbs_left`
            // (we're not zipping with `reply_surbs` directly as this would result in a move and
            // we wouldn't be able to put leftover reply surbs into the storage)
            for (fragment, reply_surb) in to_send.into_iter().zip(available_surbs.drain(..elements))
            {
                // we need to clone it because we need to keep it in memory in case we had to retransmit
                // it. And then we'd need to recreate entire ACK again.
                let chunk_clone = fragment.clone();
                let prepared_fragment = self
                    .message_preparer
                    .prepare_reply_chunk_for_sending(
                        chunk_clone,
                        topology,
                        reply_surb,
                        &self.ack_key,
                    )
                    .unwrap();

                real_messages.push(RealMessage::new(
                    prepared_fragment.mix_packet,
                    fragment.fragment_identifier(),
                ));
            }

            self.real_message_sender
                .unbounded_send(real_messages)
                .unwrap();
        } else {
            println!("nothing left to clear");
        }
    }

    async fn handle_received_surbs(
        &mut self,
        from: AnonymousSenderTag,
        mut reply_surbs: Vec<ReplySurb>,
    ) {
        println!("handling received surbs");

        // 1. make sure we have > threshold number of surbs for the given target
        let available_surbs = self.reply_storage.available_surbs(&from);
        let surbs_threshold = self.reply_storage.min_surb_threshold();

        if available_surbs < surbs_threshold {
            let to_insert = min(surbs_threshold - available_surbs, reply_surbs.len());
            self.reply_storage
                .insert_surbs(&from, &mut reply_surbs.drain(..to_insert))
        }

        // 2. if we have any pending replies, use surbs for those
        self.try_clear_pending_queue(&from, &mut reply_surbs).await;

        // 3. buffer any leftovers
        if !reply_surbs.is_empty() {
            self.reply_storage.insert_surbs(&from, reply_surbs)
        }
    }

    async fn handle_surb_request(&mut self, recipient: Recipient, amount: u32) {
        // 1. check whether the requested amount is within sane range
        // (say if it was malformed and asked for 1M surbs, we should reject it)
        // TODO:
        // 2. check whether we sent any surbs in the past to this recipient, otherwise
        // they have no business in asking for more
        // TODO:
        // 3. construct and send the surbs away
        // TODO: a lot of shared code in input message listener (this is literally `handle_fresh_message` all over again)
        // for now send Reply message with empty content, then refactor it to use the surb specific message
        let topology_permit = self.topology_access.get_read_permit().await;
        let topology = self.get_topology(&topology_permit).expect("todo");

        // split the message, attach optional reply surb
        let (split_message, reply_keys) = self
            .message_preparer
            .prepare_and_split_message(Vec::new(), amount, topology)
            .expect("somehow the topology was invalid after all!");

        log::info!("storing {} reply keys", reply_keys.len());
        self.reply_storage.insert_multiple_surb_keys(reply_keys);

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

        // welp, can't write it up easily, will do it later.
        // // tells the controller to put this into the hashmap
        // self.action_sender
        //     .unbounded_send(Action::new_insert(pending_acks))
        //     .unwrap();

        self.real_message_sender
            .unbounded_send(real_messages)
            .unwrap();
    }

    async fn handle_request(&mut self, request: ToBeNamedMessage) {
        match request {
            ToBeNamedMessage::SendReply { recipient, message } => {
                self.handle_send_reply(recipient, message).await
            }
            ToBeNamedMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
            } => self.handle_received_surbs(sender_tag, reply_surbs).await,
            ToBeNamedMessage::AdditionalSurbsRequest { recipient, amount } => {
                self.handle_surb_request(recipient, amount).await
            }
        }
    }

    // deal with shutdowns, etc, later.
    pub async fn run(&mut self) {
        while let Some(req) = self.request_receiver.next().await {
            self.handle_request(req).await
        }
    }

    // #[cfg(not(target_arch = "wasm32"))]
    // pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
    //     debug!("Started AcknowledgementListener with graceful shutdown support");
    //
    //     while !shutdown.is_shutdown() {
    //         tokio::select! {
    //             // acks = self.ack_receiver.next() => match acks {
    //             //     Some(acks) => self.handle_ack_receiver_item(acks).await,
    //             //     None => {
    //             //         log::trace!("AcknowledgementListener: Stopping since channel closed");
    //             //         break;
    //             //     }
    //             // },
    //             _ = shutdown.recv() => {
    //                 log::trace!("AcknowledgementListener: Received shutdown");
    //             }
    //         }
    //     }
    //     assert!(shutdown.is_shutdown_poll());
    //     log::debug!("AcknowledgementListener: Exiting");
    // }
    //
    // #[cfg(target_arch = "wasm32")]
    // pub(super) async fn run(&mut self) {
    //     debug!("Started AcknowledgementListener without graceful shutdown support");
    //
    //     while let Some(acks) = self.ack_receiver.next().await {
    //         self.handle_ack_receiver_item(acks).await
    //     }
    // }
}
