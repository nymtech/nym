// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::message_handler::{MessageHandler, PreparationErrorRepr};
use crate::client::replies::reply_storage::ReceivedReplySurbsMap;
use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::chunking::fragment::Fragment;
use rand::{CryptoRng, Rng};
use std::cmp::{max, min};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicUsize;
use std::time::Duration;
use thiserror::Error;
use tokio::time::Instant;

// TODO: rename

// TODO: move elsewhere and share with other bits doing surb requests
#[derive(Debug, Clone, Error)]
pub enum SurbRequestError {
    #[error("Not enough reply SURBs to send the message. We have {available} available and require {required}.")]
    NotEnoughSurbs { available: usize, required: usize },
    #[error(transparent)]
    InvalidTopology(#[from] PreparationErrorRepr),
}

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

// TODO: move when cleaning
struct PendingReply {
    next_surb_request_increment: u32,
    data: VecDeque<Fragment>,
}

impl PendingReply {
    fn new(data: Vec<Fragment>) -> Self {
        PendingReply {
            next_surb_request_increment: 0,
            data: data.into(),
        }
    }

    fn increase_surb_request_counter(&mut self, amount: u32) {
        self.next_surb_request_increment += amount
    }

    fn decrease_surb_request_counter(&mut self, amount: u32) {
        self.next_surb_request_increment = self.next_surb_request_increment.saturating_sub(amount)
    }
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
    pending_replies: HashMap<AnonymousSenderTag, PendingReply>,
    message_handler: MessageHandler<R>,
    received_reply_surbs: ReceivedReplySurbsMap,

    // TODO: move elsewhere
    min_surb_request_size: u32,
    max_surb_request_size: u32,
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
            // surbs_last_received_at: Instant::now(),

            // TODO: HARDCODED!!
            min_surb_request_size: 10,
            max_surb_request_size: 250,
        }
    }

    fn insert_pending_replies(&mut self, recipient: &AnonymousSenderTag, fragments: Vec<Fragment>) {
        if let Some(existing) = self.pending_replies.get_mut(recipient) {
            existing.data.append(&mut fragments.into())
        } else {
            self.pending_replies
                .insert(*recipient, PendingReply::new(fragments));
        }
    }

    fn increment_surb_request_counter(&mut self, recipient: &AnonymousSenderTag, amount: u32) {
        // TODO: investigate whether this failure can ever happen
        self.pending_replies
            .get_mut(recipient)
            .expect("this failure should be impossible")
            .increase_surb_request_counter(amount);
    }

    fn decrement_surb_request_counter(&mut self, recipient: &AnonymousSenderTag, amount: u32) {
        // TODO: investigate whether this failure can ever happen
        self.pending_replies
            .get_mut(recipient)
            .expect("this failure should be impossible")
            .decrease_surb_request_counter(amount);
    }

    fn reset_surb_request_counter(&mut self, recipient: &AnonymousSenderTag) {
        // TODO: investigate whether this failure can ever happen
        self.pending_replies
            .get_mut(recipient)
            .expect("this failure should be impossible")
            .next_surb_request_increment = 0;
    }

    fn surb_request_counter(&mut self, recipient: &AnonymousSenderTag) -> u32 {
        // TODO: investigate whether this failure can ever happen
        self.pending_replies
            .get_mut(recipient)
            .expect("this failure should be impossible")
            .next_surb_request_increment
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
            error!(
                "we did NOT request any extra surbs! but still used {}",
                reply_surbs.len()
            );

            if let Err(err) = self
                .message_handler
                .try_send_reply_chunks(recipient_tag, fragments, reply_surbs)
                .await
            {
                // TODO: perhaps there should be some timer here to repeat the request once topology recovers
                let (msg, returned_surbs) = err.into_inner();
                warn!("failed to send reply to {:?} - {msg}", recipient_tag);
                self.received_reply_surbs
                    .insert_maybe_surbs(&recipient_tag, returned_surbs);
            }
        } else {
            #[deprecated]
            //remove hardcoded 10
            let extra_surbs = 0;

            info!("requesting surbs from send handler");
            self.insert_pending_replies(&recipient_tag, fragments);

            // if we're running low on surbs, we should request more (unless we've already requested them)
            let mut already_requesting = self
                .received_reply_surbs
                .set_requesting_more_surbs(&recipient_tag)
                .expect("error handling");

            // TODO: revisit that dodgy if
            if already_requesting {
                warn!("we were already requesting surbs, but we shall ignore it");
                already_requesting = false;
            }

            if !already_requesting {
                let ideal_amount = extra_surbs + required_surbs as u32;
                let amount = min(
                    max(ideal_amount, self.min_surb_request_size),
                    self.max_surb_request_size,
                );

                if let Err(err) = self
                    .request_additional_reply_surbs(recipient_tag, amount)
                    .await
                {
                    error!("couldnt request additional surbs - {:?}", err);
                    self.increment_surb_request_counter(&recipient_tag, required_surbs as u32)
                    // if we failed to request surbs, increase value for the next request
                }
            }
        }
    }

    async fn request_additional_reply_surbs(
        &mut self,
        target: AnonymousSenderTag,
        mut amount: u32,
    ) -> Result<(), SurbRequestError> {
        log::info!("requesting {amount} reply surbs ...");

        let reply_surb = self
            .received_reply_surbs
            .get_reply_surb_ignoring_threshold(&target)
            .and_then(|(reply_surb, _)| reply_surb)
            .ok_or(SurbRequestError::NotEnoughSurbs {
                available: 0,
                required: 1,
            })?;

        let counter = self.surb_request_counter(&target);
        amount += counter;
        log::info!("incrementing the amount to {amount}");

        if let Err(err) = self
            .message_handler
            .try_request_additional_reply_surbs(target, reply_surb, amount)
            .await
        {
            let (msg, returned_surbs) = err.into_inner();
            warn!(
                "failed to request additional surbs from {:?} - {msg}",
                target
            );
            // TODO: perhaps there should be some timer here to repeat the request once topology recovers
            self.received_reply_surbs
                .insert_maybe_surbs(&target, returned_surbs);
            return Err(msg.into());
        }

        // decrement the counter by what we managed to request
        self.decrement_surb_request_counter(&target, amount);
        Ok(())
    }

    fn pop_at_most_pending_replies(
        &mut self,
        from: &AnonymousSenderTag,
        amount: usize,
    ) -> Option<VecDeque<Fragment>> {
        // if possible, pop all pending replies, if not, pop only entries for which we'd have a reply surb
        let total = self.pending_replies.get(from)?.data.len();
        println!("pending queue has {total} elements");
        if total < amount {
            self.pending_replies.remove(from).map(|d| d.data)
        } else {
            Some(
                self.pending_replies
                    .get_mut(from)?
                    .data
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
                // TODO: fix: this panic actually happens periodically
                panic!("empty1");
            }

            let surbs_for_reply = available_surbs.drain(..to_send_vec.len()).collect();
            if let Err(err) = self
                .message_handler
                .try_send_reply_chunks(target, to_send_vec, surbs_for_reply)
                .await
            {
                let (msg, returned_surbs) = err.into_inner();
                warn!("failed to clear pending queue for {:?} - {msg}", target);
                // TODO: perhaps there should be some timer here to repeat the request once topology recovers
                if let Some(returned_surbs) = returned_surbs {
                    self.received_reply_surbs
                        .insert_surbs(&target, returned_surbs);
                }
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

        // TODO: reset surb timer here ONLY
        self.received_reply_surbs
            .reset_surbs_last_received_at(&from);

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

        // 3. if our queue is still not empty (and we have no surb requests pending), request more surbs
        // self.request_reply_surbs_for_queue_clearing(from).await;

        // 4. buffer any leftovers
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

        // TODO: improve this dodgy loop
        let mut remaining = amount;
        while remaining > 0 {
            let to_send = min(remaining, 100);
            if let Err(err) = self
                .message_handler
                .try_send_additional_reply_surbs(recipient, to_send)
                .await
            {
                warn!("failed to send additional surbs to {recipient} - {err}");
                debug_assert!(err.into_inner().1.is_none())
            } else {
                warn!("sent {to_send} surbs");
            }

            remaining -= to_send;
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

    async fn request_reply_surbs_for_queue_clearing(&mut self, target: AnonymousSenderTag) {
        let pending = match self.pending_replies.get(&target) {
            Some(pending) => pending,
            None => {
                warn!("there are no pending replies for {:?}!", target);
                return;
            }
        };
        let queue_size = pending.data.len() as u32;
        if queue_size == 0 {
            trace!("the pending queue for {:?} is already empty", target);
            return;
        }

        let increment = pending.next_surb_request_increment;
        let request_size = min(self.max_surb_request_size, queue_size + increment);

        self.request_additional_reply_surbs(target, request_size)
            .await;
    }

    async fn inspect_stale_entries(&mut self) {
        let mut to_request = Vec::new();

        let now = Instant::now();
        for (pending_reply_target, vals) in &self.pending_replies {
            if vals.data.is_empty() {
                error!("WE'RE KEEPING EMPTY ENTRY!!")
            }

            let last_received = self
                .received_reply_surbs
                .surbs_last_received_at(pending_reply_target)
                .expect("I think this shouldnt fail? to be verified.");

            let diff = now - last_received;

            // TODO: hardcoded value
            if diff > Duration::from_secs(10) {
                warn!("we haven't received any surbs in {:?}", diff);
                to_request.push(*pending_reply_target);
            }
        }

        for pending_reply_target in to_request {
            warn!("requesting more surbs...");
            // TODO: change below
            self.request_reply_surbs_for_queue_clearing(pending_reply_target)
                .await;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started ToBeNamedPendingReplyController with graceful shutdown support");

        let polling_rate = Duration::from_secs(5);
        let mut interval_timer = tokio::time::interval(polling_rate);

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("ToBeNamedPendingReplyController: Received shutdown");
                },
                req = self.request_receiver.next() => match req {
                    Some(req) => self.handle_request(req).await,
                    None => {
                        log::trace!("ToBeNamedPendingReplyController: Stopping since channel closed");
                        break;
                    }
                },
                _ = interval_timer.tick() => {
                    self.inspect_stale_entries().await
                },
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
