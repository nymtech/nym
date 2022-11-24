// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::message_handler::{MessageHandler, PreparationError};
use crate::client::replies::reply_storage::{ReceivedReplySurbsMap, UsedSenderTags};
use client_connections::TransmissionLane;
use futures::channel::mpsc;
use futures::StreamExt;
use log::{debug, info, trace, warn};
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::ReplySurb;
use nymsphinx::chunking::fragment::Fragment;
use rand::{CryptoRng, Rng};
use std::cmp::{max, min};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tokio::time::Instant;

pub fn new_control_channels() -> (ReplyControllerSender, ReplyControllerReceiver) {
    let (tx, rx) = mpsc::unbounded();
    (tx.into(), rx)
}

#[derive(Debug, Clone)]
pub struct ReplyControllerSender(mpsc::UnboundedSender<ReplyControllerMessage>);

impl From<mpsc::UnboundedSender<ReplyControllerMessage>> for ReplyControllerSender {
    fn from(inner: mpsc::UnboundedSender<ReplyControllerMessage>) -> Self {
        ReplyControllerSender(inner)
    }
}

impl ReplyControllerSender {
    pub(crate) fn send_reply(
        &self,
        recipient: AnonymousSenderTag,
        message: Vec<u8>,
        lane: TransmissionLane,
    ) {
        self.0
            .unbounded_send(ReplyControllerMessage::SendReply {
                recipient,
                message,
                lane,
            })
            .expect("ReplyControllerReceiver has died!")
    }

    pub(crate) fn send_additional_surbs(
        &self,
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    ) {
        self.0
            .unbounded_send(ReplyControllerMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
            })
            .expect("ReplyControllerReceiver has died!")
    }

    pub(crate) fn send_additional_surbs_request(&self, recipient: Recipient, amount: u32) {
        self.0
            .unbounded_send(ReplyControllerMessage::AdditionalSurbsRequest {
                recipient: Box::new(recipient),
                amount,
            })
            .expect("ReplyControllerReceiver has died!")
    }
}

pub type ReplyControllerReceiver = mpsc::UnboundedReceiver<ReplyControllerMessage>;

#[derive(Debug)]
pub enum ReplyControllerMessage {
    SendReply {
        recipient: AnonymousSenderTag,
        message: Vec<u8>,
        lane: TransmissionLane,
    },

    AdditionalSurbs {
        sender_tag: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    },

    // Should this also be handled in here? it's technically a completely different side of the pipe
    // let's see how it works when combined, might split it before creating PR
    AdditionalSurbsRequest {
        recipient: Box<Recipient>,
        amount: u32,
    },
}

// the purpose of this task:
// - buffers split messages from input message listener if there were insufficient surbs to send them
// - upon getting extra surbs, resends them
// - so I guess it will handle all 'RepliableMessage' and requests from 'ReplyMessage'
// - replies to "give additional surbs" requests
// - will reply to future heartbeats

// TODO: this should be split into ingress and egress controllers
// because currently its trying to perform two distinct jobs
pub struct ReplyController<R> {
    // TODO: incorporate that field at some point
    // and use binomial distribution to determine the expected required number
    // of surbs required to send the message through
    // expected_reliability: f32,
    request_receiver: ReplyControllerReceiver,
    pending_replies: HashMap<AnonymousSenderTag, VecDeque<Fragment>>,
    message_handler: MessageHandler<R>,
    received_reply_surbs: ReceivedReplySurbsMap,
    tag_storage: UsedSenderTags,

    min_surb_request_size: u32,
    max_surb_request_size: u32,
    maximum_allowed_reply_surb_request_size: u32,
    max_surb_waiting_period: Duration,
}

impl<R> ReplyController<R>
where
    R: CryptoRng + Rng,
{
    pub(crate) fn new(
        message_handler: MessageHandler<R>,
        received_reply_surbs: ReceivedReplySurbsMap,
        tag_storage: UsedSenderTags,
        request_receiver: ReplyControllerReceiver,
        min_surb_request_size: u32,
        max_surb_request_size: u32,
        maximum_allowed_reply_surb_request_size: u32,
        max_surb_waiting_period: Duration,
    ) -> Self {
        ReplyController {
            request_receiver,
            pending_replies: Default::default(),
            message_handler,
            received_reply_surbs,
            tag_storage,
            min_surb_request_size,
            max_surb_request_size,
            maximum_allowed_reply_surb_request_size,
            max_surb_waiting_period,
        }
    }

    fn insert_pending_replies(&mut self, recipient: &AnonymousSenderTag, fragments: Vec<Fragment>) {
        if let Some(existing) = self.pending_replies.get_mut(recipient) {
            existing.append(&mut fragments.into())
        } else {
            self.pending_replies.insert(*recipient, fragments.into());
        }
    }

    fn should_request_more_surbs(&self, target: &AnonymousSenderTag) -> bool {
        trace!("checking if we should request more surbs from {:?}", target);

        // if we don't have any information associated with this target,
        // then we definitely don't want any more surbs
        let queue_size = match self.pending_replies.get(target) {
            Some(pending_queue) => pending_queue.len(),
            None => return false,
        };

        let available_surbs = self.received_reply_surbs.available_surbs(target);
        let pending_surbs = self.received_reply_surbs.pending_reception(target) as usize;
        let min_surbs_threshold = self.received_reply_surbs.min_surb_threshold();
        let max_surbs_threshold = self.received_reply_surbs.max_surb_threshold();

        println!("queue size: {queue_size}, available surbs: {available_surbs} pending surbs: {pending_surbs} threshold range: {min_surbs_threshold}..{max_surbs_threshold}");

        (pending_surbs + available_surbs) < max_surbs_threshold
            && (pending_surbs + available_surbs) < (queue_size + min_surbs_threshold)
    }

    async fn handle_send_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
    ) {
        if !self.received_reply_surbs.contains_surbs_for(&recipient_tag) {
            warn!("received reply request for {:?} but we don't have any surbs stored for that recipient!", recipient_tag);
            return;
        }

        // TODO: lower to debug/trace
        info!("handling reply to {:?}", recipient_tag);
        let fragments = self.message_handler.split_reply_message(data);

        let required_surbs = fragments.len();
        info!("This reply requires {:?} SURBs", required_surbs);

        // TODO: edge case:
        // we're making a lot of requests and have to request a lot of surbs
        // (but at some point we run out of surbs for surb requests)

        let (surbs, _surbs_left) = self
            .received_reply_surbs
            .get_reply_surbs(&recipient_tag, required_surbs);

        if let Some(reply_surbs) = surbs {
            if let Err(err) = self
                .message_handler
                .try_send_reply_chunks(recipient_tag, fragments, reply_surbs, lane)
                .await
            {
                // TODO: perhaps there should be some timer here to repeat the request once topology recovers
                let err = err.return_unused_surbs(&self.received_reply_surbs, &recipient_tag);
                warn!("failed to send reply to {:?} - {err}", recipient_tag);
            }
        } else {
            // we don't have enough surbs for this reply

            info!("requesting surbs from send handler");
            self.insert_pending_replies(&recipient_tag, fragments);

            if self.should_request_more_surbs(&recipient_tag) {
                self.request_reply_surbs_for_queue_clearing(recipient_tag)
                    .await;
            }
        }
    }

    async fn request_additional_reply_surbs(
        &mut self,
        target: AnonymousSenderTag,
        amount: u32,
    ) -> Result<(), PreparationError> {
        info!("requesting {amount} reply surbs ...");

        let reply_surb = self
            .received_reply_surbs
            .get_reply_surb_ignoring_threshold(&target)
            .and_then(|(reply_surb, _)| reply_surb)
            .ok_or(PreparationError::NotEnoughSurbs {
                available: 0,
                required: 1,
            })?;

        if let Err(err) = self
            .message_handler
            .try_request_additional_reply_surbs(target, reply_surb, amount)
            .await
        {
            let err = err.return_unused_surbs(&self.received_reply_surbs, &target);
            warn!(
                "failed to request additional surbs from {:?} - {err}",
                target
            );
            // TODO: perhaps there should be some timer here to repeat the request once topology recovers
            return Err(err);
        } else {
            self.received_reply_surbs
                .increment_pending_reception(&target, amount);
        }

        Ok(())
    }

    fn pop_at_most_pending_replies(
        &mut self,
        from: &AnonymousSenderTag,
        amount: usize,
    ) -> Option<VecDeque<Fragment>> {
        // if possible, pop all pending replies, if not, pop only entries for which we'd have a reply surb
        let total = self.pending_replies.get(from)?.len();
        println!("pending queue has {total} elements");
        if total == 0 {
            return None;
        }
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

    async fn try_clear_pending_queue(&mut self, target: AnonymousSenderTag) {
        println!("trying to clear pending queue");
        let available_surbs = self.received_reply_surbs.available_surbs(&target);
        let min_surbs_threshold = self.received_reply_surbs.min_surb_threshold();

        let max_to_clear = if available_surbs > min_surbs_threshold {
            available_surbs - min_surbs_threshold
        } else {
            println!("we don't have enough surbs for queue clearing...");
            return;
        };
        println!("we can clear up to {max_to_clear} entries");

        // we're guaranteed to not get more entries than we have reply surbs for
        if let Some(to_send) = self.pop_at_most_pending_replies(&target, max_to_clear) {
            // TODO: optimise: we're cloning the fragments every time to re-insert them into the buffer in case of failure
            let to_send_vec = to_send.into_iter().collect::<Vec<_>>();

            if to_send_vec.is_empty() {
                // TODO: fix: this panic actually happens periodically
                panic!("empty1");
            }

            let (surbs_for_reply, _) = self
                .received_reply_surbs
                .get_reply_surbs(&target, to_send_vec.len());
            let surbs_for_reply =
                surbs_for_reply.expect("is is possible for this to ever show up?");

            if let Err(err) = self
                .message_handler
                .try_send_reply_chunks(
                    target,
                    to_send_vec,
                    surbs_for_reply,
                    TransmissionLane::General,
                )
                .await
            {
                let err = err.return_unused_surbs(&self.received_reply_surbs, &target);
                warn!("failed to clear pending queue for {:?} - {err}", target);
                // TODO: perhaps there should be some timer here to repeat the request once topology recovers
            }
        } else {
            println!("nothing left to clear");
        }
    }

    async fn handle_received_surbs(
        &mut self,
        from: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    ) {
        println!("handling received surbs");

        // clear the requesting flag since we should have been asking for surbs
        self.received_reply_surbs
            .reset_surbs_last_received_at(&from);
        if from_surb_request {
            self.received_reply_surbs
                .decrement_pending_reception(&from, reply_surbs.len() as u32);
        }

        // store received surbs
        self.received_reply_surbs.insert_surbs(&from, reply_surbs);

        // use as many as we can for clearing pending queue
        self.try_clear_pending_queue(from).await;

        // if we have to, request more
        if self.should_request_more_surbs(&from) {
            self.request_reply_surbs_for_queue_clearing(from).await;
        }
    }

    async fn handle_surb_request(&mut self, recipient: Recipient, mut amount: u32) {
        // 1. check whether we sent any surbs in the past to this recipient, otherwise
        // they have no business in asking for more
        if !self.tag_storage.exists(&recipient) {
            warn!("{recipient} asked us for reply SURBs even though we never sent them any anonymous messages before!");
            return;
        }

        // 2. check whether the requested amount is within sane range
        if amount > self.maximum_allowed_reply_surb_request_size {
            warn!("The requested reply surb amount is larger than our maximum allowed ({amount} > {}). Lowering it to a more sane value...", self.maximum_allowed_reply_surb_request_size);
            amount = self.maximum_allowed_reply_surb_request_size;
        }

        // 3. construct and send the surbs away
        // (send them in smaller batches to make the experience a bit smoother
        let mut remaining = amount;
        while remaining > 0 {
            let to_send = min(remaining, 100);
            if let Err(err) = self
                .message_handler
                .try_send_additional_reply_surbs(recipient, to_send)
                .await
            {
                warn!("failed to send additional surbs to {recipient} - {err}");
            } else {
                warn!("sent {to_send} surbs");
            }

            remaining -= to_send;
        }
    }

    async fn handle_request(&mut self, request: ReplyControllerMessage) {
        match request {
            ReplyControllerMessage::SendReply {
                recipient,
                message,
                lane,
            } => self.handle_send_reply(recipient, message, lane).await,
            ReplyControllerMessage::AdditionalSurbs {
                sender_tag,
                reply_surbs,
                from_surb_request,
            } => {
                self.handle_received_surbs(sender_tag, reply_surbs, from_surb_request)
                    .await
            }
            ReplyControllerMessage::AdditionalSurbsRequest { recipient, amount } => {
                self.handle_surb_request(*recipient, amount).await
            }
        }
    }

    async fn request_reply_surbs_for_queue_clearing(&mut self, target: AnonymousSenderTag) {
        warn!("requesting surbs for queue clearing");

        let pending = match self.pending_replies.get(&target) {
            Some(pending) => pending,
            None => {
                warn!("there are no pending replies for {:?}!", target);
                return;
            }
        };
        let queue_size = pending.len() as u32;
        if queue_size == 0 {
            trace!("the pending queue for {:?} is already empty", target);
            return;
        }

        let request_size = min(
            self.max_surb_request_size,
            max(queue_size, self.min_surb_request_size),
        );

        if let Err(err) = self
            .request_additional_reply_surbs(target, request_size)
            .await
        {
            warn!("failed to request additional surbs... - {err}")
        }
    }

    async fn inspect_stale_entries(&mut self) {
        let mut to_request = Vec::new();

        let now = Instant::now();
        for (pending_reply_target, vals) in &self.pending_replies {
            if vals.is_empty() {
                // TODO: remove it from the map before getting here
                continue;
            }

            let last_received = self
                .received_reply_surbs
                .surbs_last_received_at(pending_reply_target)
                .expect("I think this shouldnt fail? to be verified.");

            let diff = now - last_received;

            if diff > self.max_surb_waiting_period {
                warn!("We haven't received any surbs in {:?} from {:?}. Going to explicitly ask for more", diff, pending_reply_target);
                to_request.push(*pending_reply_target);
            }
        }

        for pending_reply_target in to_request {
            self.request_reply_surbs_for_queue_clearing(pending_reply_target)
                .await;
            self.received_reply_surbs
                .reset_pending_reception(&pending_reply_target)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started ReplyController with graceful shutdown support");

        let polling_rate = Duration::from_secs(5);
        let mut interval_timer = tokio::time::interval(polling_rate);

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("ReplyController: Received shutdown");
                },
                req = self.request_receiver.next() => match req {
                    Some(req) => self.handle_request(req).await,
                    None => {
                        log::trace!("ReplyController: Stopping since channel closed");
                        break;
                    }
                },
                _ = interval_timer.tick() => {
                    self.inspect_stale_entries().await
                },
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("ReplyController: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn run(&mut self) {
        debug!("Started ReplyController without graceful shutdown support");

        while let Some(req) = self.request_receiver.next().await {
            self.handle_request(req).await
        }
    }
}
