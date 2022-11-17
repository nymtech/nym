// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::message_handler::MessageHandler;
use crate::client::replies::reply_storage::ReceivedReplySurbsMap;
use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, error, info, warn};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::chunking::fragment::Fragment;
use rand::{CryptoRng, Rng};
use std::cmp::min;
use std::collections::{HashMap, VecDeque};

// TODO: rename

pub fn new_control_channels() -> (ToBeNamedSender, ToBeNamedReceiver) {
    let (tx, rx) = mpsc::unbounded();
    (tx.into(), rx)
}

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
        from_surb_request: bool,
    ) {
        self.0
            .unbounded_send(ToBeNamedMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
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
        from_surb_request: bool,
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
    // expected_reliability: f32,
    request_receiver: ToBeNamedReceiver,
    pending_replies: HashMap<AnonymousSenderTag, VecDeque<Fragment>>,
    message_handler: MessageHandler<R>,
    received_reply_surbs: ReceivedReplySurbsMap,
}

impl<R> ToBeNamedPendingReplyController<R>
where
    R: CryptoRng + Rng,
{
    // TODO: don't make it public
    pub(crate) fn new(
        message_handler: MessageHandler<R>,
        received_reply_surbs: ReceivedReplySurbsMap,
        request_receiver: ToBeNamedReceiver,
    ) -> Self {
        ToBeNamedPendingReplyController {
            request_receiver,
            pending_replies: Default::default(),
            message_handler,
            received_reply_surbs,
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
        if !self.received_reply_surbs.contains_surbs_for(&recipient_tag) {
            warn!("received reply request for {:?} but we don't have any surbs stored for that recipient!", recipient_tag);
            return;
        }

        // TODO: lower to debug/trace
        info!("handling reply to {:?}", recipient_tag);
        let fragments = self.message_handler.split_reply_message(data);

        let required_surbs = fragments.len();
        info!("This reply requires {:?} SURBs", fragments.len());

        // TODO: edge case:
        // we're making a lot of requests and have to request a lot of surbs
        // (but at some point we run out of surbs for surb requests)

        let (surbs, surbs_left) = self
            .received_reply_surbs
            .get_reply_surbs(&recipient_tag, fragments.len());

        if let Some(reply_surbs) = surbs {
            if let Err(returned_surbs) = self
                .message_handler
                .try_send_reply_chunks(recipient_tag, fragments, reply_surbs)
                .await
            {
                warn!("failed to send reply to {:?}", recipient_tag);
                // TODO: perhaps there should be some timer here to repeat the request once topology recovers
                self.received_reply_surbs
                    .insert_surbs(&recipient_tag, returned_surbs);
            }
        } else {
            #[deprecated]
            //remove hardcoded 10
            let extra_surbs = 10;

            info!("requesting surbs from send handler");
            self.insert_pending_replies(&recipient_tag, fragments);

            // if we're running low on surbs, we should request more (unless we've already requested them)
            let mut already_requesting = self
                .received_reply_surbs
                .set_requesting_more_surbs(&recipient_tag)
                .expect("error handling");

            if already_requesting {
                warn!("we were already requesting surbs, but we shall ignore it");
                already_requesting = false;
            }

            if !already_requesting {
                self.request_additional_reply_surbs(
                    recipient_tag,
                    extra_surbs + required_surbs as u32,
                )
                .await;
                // .expect("this temporary error handling HAS TO go")
            }
        }
    }

    async fn request_additional_reply_surbs(
        &mut self,
        target: AnonymousSenderTag,
        amount: u32,
    ) -> Option<()> {
        log::info!("requesting {amount} reply surbs ...");

        let (reply_surb, _) = self
            .received_reply_surbs
            .get_reply_surb_ignoring_threshold(&target)?;
        let reply_surb = reply_surb?;

        if let Err(returned_surb) = self
            .message_handler
            .try_request_additional_reply_surbs(target, reply_surb, amount)
            .await
        {
            warn!("failed to request additional surbs from {:?}", target);
            // TODO: perhaps there should be some timer here to repeat the request once topology recovers
            self.received_reply_surbs
                .insert_surb(&target, returned_surb);
        }

        // TODO: that's a really terrible return type.
        Some(())
    }

    fn pop_at_most_pending_replies(
        &mut self,
        from: &AnonymousSenderTag,
        amount: usize,
    ) -> Option<VecDeque<Fragment>> {
        // if possible, pop all pending replies, if not, pop only entries for which we'd have a reply surb
        let total = self.pending_replies.get(from)?.len();
        println!("pending queue has {total} elements");
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
        target: AnonymousSenderTag,
        available_surbs: &mut Vec<ReplySurb>,
    ) {
        println!("trying to clear pending queue");
        let surbs_left = available_surbs.len();
        if surbs_left == 0 {
            println!("we have no surbs...");
            return;
        }

        println!("we have {} surbs on hand", surbs_left);

        // we're guaranteed to not get more entries than we have reply surbs for
        if let Some(to_send) = self.pop_at_most_pending_replies(&target, surbs_left) {
            // TODO: optimise: we're cloning the fragments every time to re-insert them into the buffer in case of failure
            let to_send_vec = to_send.into_iter().collect::<Vec<_>>();

            if to_send_vec.is_empty() {
                panic!("empty1");
            }

            let surbs_for_reply = available_surbs.drain(..to_send_vec.len()).collect();
            if let Err(returned_surbs) = self
                .message_handler
                .try_send_reply_chunks(target, to_send_vec, surbs_for_reply)
                .await
            {
                warn!("failed to clear pending queue for {:?}", target);
                // TODO: perhaps there should be some timer here to repeat the request once topology recovers
                self.received_reply_surbs
                    .insert_surbs(&target, returned_surbs);
            }
        } else {
            println!("nothing left to clear");
        }
    }

    async fn handle_received_surbs(
        &mut self,
        from: AnonymousSenderTag,
        mut reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    ) {
        println!("handling received surbs");

        // clear the requesting flag since we should have been asking for surbs
        if from_surb_request
            && self
                .received_reply_surbs
                .clear_requesting_more_surbs(&from)
                .is_none()
        {
            error!("received more surbs without asking for them! - what the hell?")
        }

        // 1. make sure we have > threshold number of surbs for the given target
        let available_surbs = self.received_reply_surbs.available_surbs(&from);
        let surbs_threshold = self.received_reply_surbs.min_surb_threshold();

        if available_surbs < surbs_threshold {
            let to_insert = min(surbs_threshold - available_surbs, reply_surbs.len());
            self.received_reply_surbs
                .insert_surbs(&from, &mut reply_surbs.drain(..to_insert))
        }

        // 2. if we have any pending replies, use surbs for those
        self.try_clear_pending_queue(from, &mut reply_surbs).await;

        // 3. buffer any leftovers
        if !reply_surbs.is_empty() {
            self.received_reply_surbs.insert_surbs(&from, reply_surbs)
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
        if self
            .message_handler
            .try_send_normal_message(recipient, Vec::new(), amount)
            .await
            .is_none()
        {
            warn!("failed to send additional surbs to {}", recipient)
        }
    }

    async fn handle_request(&mut self, request: ToBeNamedMessage) {
        match request {
            ToBeNamedMessage::SendReply { recipient, message } => {
                self.handle_send_reply(recipient, message).await
            }
            ToBeNamedMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
            } => {
                self.handle_received_surbs(sender_tag, reply_surbs, from_surb_request)
                    .await
            }
            ToBeNamedMessage::AdditionalSurbsRequest { recipient, amount } => {
                self.handle_surb_request(recipient, amount).await
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started ToBeNamedPendingReplyController with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                req = self.request_receiver.next() => match req {
                    Some(req) => self.handle_request(req).await,
                    None => {
                        log::trace!("ToBeNamedPendingReplyController: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv() => {
                    log::trace!("ToBeNamedPendingReplyController: Received shutdown");
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("ToBeNamedPendingReplyController: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn run(&mut self) {
        debug!("Started ToBeNamedPendingReplyController without graceful shutdown support");

        while let Some(req) = self.request_receiver.next().await {
            self.handle_request(req).await
        }
    }
}
