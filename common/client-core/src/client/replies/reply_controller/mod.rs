// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use crate::client::real_messages_control::message_handler::{MessageHandler, PreparationError};
use crate::client::replies::reply_storage::CombinedReplyStorage;
use futures::channel::oneshot;
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::anonymous_replies::ReplySurb;
use nym_sphinx::chunking::fragment::{Fragment, FragmentIdentifier};
use nym_task::connections::{ConnectionId, TransmissionLane};
use rand::{CryptoRng, Rng};
use std::cmp::{max, min};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Weak};
use std::time::Duration;
use time::OffsetDateTime;

use crate::client::helpers::new_interval_stream;
use crate::client::transmission_buffer::TransmissionBuffer;
use crate::config;
pub(crate) use requests::{ReplyControllerMessage, ReplyControllerReceiver, ReplyControllerSender};

pub mod requests;

// this is still left as a separate config so I wouldn't need to replace it everywhere
// plus its not unreasonable to think that we might need something outside config::ReplySurbs struct
pub struct Config {
    reply_surbs: config::ReplySurbs,
}

impl Config {
    pub(crate) fn new(reply_surbs_cfg: config::ReplySurbs) -> Self {
        Self {
            reply_surbs: reply_surbs_cfg,
        }
    }
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
    config: Config,

    // TODO: incorporate that field at some point
    // and use binomial distribution to determine the expected required number
    // of surbs required to send the message through
    // expected_reliability: f32,
    request_receiver: ReplyControllerReceiver,
    pending_replies: HashMap<AnonymousSenderTag, TransmissionBuffer<Fragment>>,

    /// Retransmission packets that have already timed out and are waiting for additional reply SURBs
    /// so that they could be sent back to the network. Once we receive more SURBs, we should send them ASAP.
    // TODO: when purging stale entries, we must take extra care to also purge all pending ACK data!!
    pending_retransmissions:
        HashMap<AnonymousSenderTag, BTreeMap<FragmentIdentifier, Weak<PendingAcknowledgement>>>,

    message_handler: MessageHandler<R>,
    full_reply_storage: CombinedReplyStorage,
}

impl<R> ReplyController<R>
where
    R: CryptoRng + Rng,
{
    pub(crate) fn new(
        config: Config,
        message_handler: MessageHandler<R>,
        full_reply_storage: CombinedReplyStorage,
        request_receiver: ReplyControllerReceiver,
    ) -> Self {
        ReplyController {
            config,
            request_receiver,
            pending_replies: HashMap::new(),
            pending_retransmissions: HashMap::new(),
            message_handler,
            full_reply_storage,
        }
    }

    fn insert_pending_replies<I: IntoIterator<Item = Fragment>>(
        &mut self,
        recipient: &AnonymousSenderTag,
        fragments: I,
        lane: TransmissionLane,
    ) {
        self.pending_replies
            .entry(*recipient)
            .or_insert_with(TransmissionBuffer::new)
            .store(&lane, fragments)
    }

    fn re_insert_pending_replies(
        &mut self,
        recipient: &AnonymousSenderTag,
        fragments: Vec<(TransmissionLane, Fragment)>,
    ) {
        // the buffer should ALWAYS exist at this point, if it doesn't, it's a bug...
        self.pending_replies
            .entry(*recipient)
            .or_insert_with(TransmissionBuffer::new)
            .store_multiple(fragments)
    }

    fn re_insert_pending_retransmission(
        &mut self,
        recipient: &AnonymousSenderTag,
        data: Vec<Arc<PendingAcknowledgement>>,
    ) {
        // the underlying entry MUST exist as we've just got data from there
        let map_entry = self
            .pending_retransmissions
            .get_mut(recipient)
            .expect("our pending retransmission entry is somehow gone!");

        for pending in data {
            // if it's 0, we don't need to do anything - we just got that ack!
            if Arc::strong_count(&pending) > 1 {
                let id = pending.inner_fragment_identifier();
                let downgraded = Arc::downgrade(&pending);
                map_entry.insert(id, downgraded);
            }
        }
    }

    fn should_request_more_surbs(&self, target: &AnonymousSenderTag) -> bool {
        trace!("checking if we should request more surbs from {:?}", target);

        let pending_queue_size = self
            .pending_replies
            .get(target)
            .map(|pending_queue| pending_queue.total_size())
            .unwrap_or_default();

        let retransmission_queue = self
            .pending_retransmissions
            .get(target)
            .map(|pending_queue| pending_queue.len())
            .unwrap_or_default();

        let total_queue = pending_queue_size + retransmission_queue;

        // simple as that - there's absolutely nothing to retransmit
        if total_queue == 0 {
            return false;
        }

        let available_surbs = self
            .full_reply_storage
            .surbs_storage_ref()
            .available_surbs(target);
        let pending_surbs = self
            .full_reply_storage
            .surbs_storage_ref()
            .pending_reception(target) as usize;
        let min_surbs_threshold = self
            .full_reply_storage
            .surbs_storage_ref()
            .min_surb_threshold();
        let max_surbs_threshold = self
            .full_reply_storage
            .surbs_storage_ref()
            .max_surb_threshold();

        debug!("total queue size: {total_queue} = pending data {pending_queue_size} + pending retransmission {retransmission_queue}, available surbs: {available_surbs} pending surbs: {pending_surbs} threshold range: {min_surbs_threshold}..{max_surbs_threshold}");

        (pending_surbs + available_surbs) < max_surbs_threshold
            && (pending_surbs + available_surbs) < (total_queue + min_surbs_threshold)
    }

    async fn handle_send_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
    ) {
        if !self
            .full_reply_storage
            .surbs_storage_ref()
            .contains_surbs_for(&recipient_tag)
        {
            warn!("received reply request for {:?} but we don't have any surbs stored for that recipient!", recipient_tag);
            return;
        }

        trace!("handling reply to {:?}", recipient_tag);
        let mut fragments = self.message_handler.split_reply_message(data);
        let total_size = fragments.len();
        trace!("This reply requires {:?} SURBs", total_size);

        let available_surbs = self
            .full_reply_storage
            .surbs_storage_ref()
            .available_surbs(&recipient_tag);
        let min_surbs_threshold = self
            .full_reply_storage
            .surbs_storage_ref()
            .min_surb_threshold();

        let max_to_send = if available_surbs > min_surbs_threshold {
            min(fragments.len(), available_surbs - min_surbs_threshold)
        } else {
            0
        };

        if max_to_send > 0 {
            let (surbs, _surbs_left) = self
                .full_reply_storage
                .surbs_storage_ref()
                .get_reply_surbs(&recipient_tag, max_to_send);

            if let Some(reply_surbs) = surbs {
                let to_send = fragments.drain(..max_to_send).collect::<Vec<_>>();
                if let Err(err) = self
                    .message_handler
                    .try_send_reply_chunks_on_lane(
                        recipient_tag,
                        to_send.clone(),
                        reply_surbs,
                        lane,
                    )
                    .await
                {
                    let err = err.return_unused_surbs(
                        self.full_reply_storage.surbs_storage_ref(),
                        &recipient_tag,
                    );
                    warn!("failed to send reply to {recipient_tag}: {err}");
                    self.insert_pending_replies(&recipient_tag, to_send, lane);
                }
            }
        }

        // if there's leftover data we didn't send because we didn't have enough (or any) surbs - buffer it
        if !fragments.is_empty() {
            self.insert_pending_replies(&recipient_tag, fragments, lane);
        }

        if self.should_request_more_surbs(&recipient_tag) {
            self.request_reply_surbs_for_queue_clearing(recipient_tag)
                .await;
        }
    }

    async fn request_additional_reply_surbs(
        &mut self,
        target: AnonymousSenderTag,
        amount: u32,
    ) -> Result<(), PreparationError> {
        let reply_surb = self
            .full_reply_storage
            .surbs_storage_ref()
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
            let err = err.return_unused_surbs(self.full_reply_storage.surbs_storage_ref(), &target);
            warn!(
                "failed to request additional surbs from {:?} - {err}",
                target
            );
            return Err(err);
        } else {
            self.full_reply_storage
                .surbs_storage_ref()
                .increment_pending_reception(&target, amount);
        }

        Ok(())
    }

    async fn try_clear_pending_retransmission(&mut self, target: AnonymousSenderTag) {
        trace!("trying to clear pending retransmission queue");
        let available_surbs = self
            .full_reply_storage
            .surbs_storage_ref()
            .available_surbs(&target);
        let min_surbs_threshold = self
            .full_reply_storage
            .surbs_storage_ref()
            .min_surb_threshold();

        let max_to_clear = if available_surbs > min_surbs_threshold {
            available_surbs - min_surbs_threshold
        } else {
            trace!("we don't have enough surbs for retransmission queue clearing...");
            return;
        };
        trace!("we can clear up to {max_to_clear} entries");

        let Some(pending) = self.pending_retransmissions.get_mut(&target) else {
            trace!("there are no pending retransmissions for {target}!");
            return;
        };

        let mut to_take = Vec::new();

        while to_take.len() < max_to_clear {
            if let Some((_, data)) = pending.pop_first() {
                // no need to do anything if we failed to upgrade the reference,
                // it means we got the ack while the data was waiting in the queue
                if let Some(upgraded) = data.upgrade() {
                    to_take.push(upgraded)
                }
            } else {
                // our map is empty!
                break;
            }
        }

        if to_take.is_empty() {
            // no need to do anything
            return;
        }

        let (surbs_for_reply, _) = self
            .full_reply_storage
            .surbs_storage_ref()
            .get_reply_surbs(&target, to_take.len());

        let Some(surbs_for_reply) = surbs_for_reply else {
            error!("somehow different task has stolen our reply surbs! - this should have been impossible");
            self.re_insert_pending_retransmission(&target, to_take);
            return;
        };

        let to_send_vec = to_take.iter().map(|ack| ack.fragment_data()).collect();

        let prepared_fragments = match self
            .message_handler
            .prepare_reply_chunks_for_sending(to_send_vec, surbs_for_reply)
            .await
        {
            Ok(prepared) => prepared,
            Err(err) => {
                let err =
                    err.return_unused_surbs(self.full_reply_storage.surbs_storage_ref(), &target);
                self.re_insert_pending_retransmission(&target, to_take);

                warn!(
                    "failed to clear pending retransmission queue for {:?} - {err}",
                    target
                );
                return;
            }
        };

        // we can't fail at this point, so drop all references to acks so that timer updates wouldn't blow up
        drop(to_take);

        self.message_handler
            .send_retransmission_reply_chunks(prepared_fragments, TransmissionLane::Retransmission)
            .await;
    }

    fn pop_at_most_pending_replies(
        &mut self,
        from: &AnonymousSenderTag,
        amount: usize,
    ) -> Option<Vec<(TransmissionLane, Fragment)>> {
        // if possible, pop all pending replies, if not, pop only entries for which we'd have a reply surb
        let total = self.pending_replies.get(from)?.total_size();
        trace!("pending queue has {total} elements");
        if total == 0 {
            return None;
        }
        self.pending_replies
            .get_mut(from)?
            .pop_at_most_n_next_messages_at_random(amount)
    }

    async fn try_clear_pending_queue(&mut self, target: AnonymousSenderTag) {
        trace!("trying to clear pending queue");
        let available_surbs = self
            .full_reply_storage
            .surbs_storage_ref()
            .available_surbs(&target);
        let min_surbs_threshold = self
            .full_reply_storage
            .surbs_storage_ref()
            .min_surb_threshold();

        let max_to_clear = if available_surbs > min_surbs_threshold {
            available_surbs - min_surbs_threshold
        } else {
            trace!("we don't have enough surbs for queue clearing...");
            return;
        };
        trace!("we can clear up to {max_to_clear} entries");

        // we're guaranteed to not get more entries than we have reply surbs for
        if let Some(to_send) = self.pop_at_most_pending_replies(&target, max_to_clear) {
            let to_send_clone = to_send.clone();

            if to_send_clone.is_empty() {
                panic!(
                    "please let the devs know if you ever see this message (reply_controller.rs)"
                );
            }

            let (surbs_for_reply, _) = self
                .full_reply_storage
                .surbs_storage_ref()
                .get_reply_surbs(&target, to_send_clone.len());

            let Some(surbs_for_reply) = surbs_for_reply else {
                error!("somehow different task has stolen our reply surbs! - this should have been impossible");
                self.re_insert_pending_replies(&target, to_send);
                return;
            };

            if let Err(err) = self
                .message_handler
                .try_send_reply_chunks(target, to_send_clone, surbs_for_reply)
                .await
            {
                let err =
                    err.return_unused_surbs(self.full_reply_storage.surbs_storage_ref(), &target);
                self.re_insert_pending_replies(&target, to_send);
                warn!("failed to clear pending queue for {:?} - {err}", target);
            }
        } else {
            trace!("the pending queue is empty");
        }
    }

    async fn handle_received_surbs(
        &mut self,
        from: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurb>,
        from_surb_request: bool,
    ) {
        trace!("handling received surbs");

        // clear the requesting flag since we should have been asking for surbs
        self.full_reply_storage
            .surbs_storage_ref()
            .reset_surbs_last_received_at(&from);
        if from_surb_request {
            self.full_reply_storage
                .surbs_storage_ref()
                .decrement_pending_reception(&from, reply_surbs.len() as u32);
        }

        // store received surbs
        self.full_reply_storage
            .surbs_storage_ref()
            .insert_surbs(&from, reply_surbs);

        // use as many as we can for clearing pending retransmission queue
        self.try_clear_pending_retransmission(from).await;

        // use as many as we can for clearing pending 'normal' queue
        self.try_clear_pending_queue(from).await;

        // if we have to, request more
        if self.should_request_more_surbs(&from) {
            self.request_reply_surbs_for_queue_clearing(from).await;
        }
    }

    async fn handle_surb_request(&mut self, recipient: Recipient, mut amount: u32) {
        // 1. check whether we sent any surbs in the past to this recipient, otherwise
        // they have no business in asking for more
        if !self
            .full_reply_storage
            .tags_storage_ref()
            .exists(&recipient)
        {
            warn!("{recipient} asked us for reply SURBs even though we never sent them any anonymous messages before!");
            return;
        }

        // 2. check whether the requested amount is within sane range
        if amount
            > self
                .config
                .reply_surbs
                .maximum_allowed_reply_surb_request_size
        {
            warn!("The requested reply surb amount is larger than our maximum allowed ({amount} > {}). Lowering it to a more sane value...", self.config.reply_surbs.maximum_allowed_reply_surb_request_size);
            amount = self
                .config
                .reply_surbs
                .maximum_allowed_reply_surb_request_size;
        }

        // 3. construct and send the surbs away
        // (send them in smaller batches to make the experience a bit smoother
        let mut remaining = amount;
        while remaining > 0 {
            let to_send = min(remaining, 100);
            if let Err(err) = self
                .message_handler
                .try_send_additional_reply_surbs(
                    recipient,
                    to_send,
                    nym_sphinx::params::PacketType::Mix,
                    self.config.reply_surbs.surb_mix_hops,
                )
                .await
            {
                warn!("failed to send additional surbs to {recipient} - {err}");
            } else {
                trace!("sent {to_send} reply SURBs to {recipient}");
            }

            remaining -= to_send;
        }
    }

    fn buffer_pending_ack(
        &mut self,
        recipient: AnonymousSenderTag,
        ack_ref: Arc<PendingAcknowledgement>,
        weak_ack_ref: Weak<PendingAcknowledgement>,
    ) {
        let frag_id = ack_ref.inner_fragment_identifier();
        if let Some(existing) = self.pending_retransmissions.get_mut(&recipient) {
            if let Entry::Vacant(e) = existing.entry(frag_id) {
                e.insert(weak_ack_ref);
            } else {
                warn!("we're already trying to retransmit {frag_id}. We must be really behind in surbs!");
            }
        } else {
            let mut inner = BTreeMap::new();
            inner.insert(frag_id, weak_ack_ref);
            self.pending_retransmissions.insert(recipient, inner);
        }
    }

    async fn handle_reply_retransmission(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        timed_out_ack: Weak<PendingAcknowledgement>,
        extra_surbs_request: bool,
    ) {
        // seems we got the ack in the end
        let ack_ref = match timed_out_ack.upgrade() {
            Some(ack) => ack,
            None => {
                debug!("we received the ack for one of the reply packets as we were putting it in the retransmission queue");
                return;
            }
        };

        // if this is retransmission for obtaining additional reply surbs,
        // we can dip below the storage threshold
        let (maybe_reply_surb, _) = if extra_surbs_request {
            self.full_reply_storage
                .surbs_storage_ref()
                .get_reply_surb_ignoring_threshold(&recipient_tag)
        } else {
            self.full_reply_storage
                .surbs_storage_ref()
                .get_reply_surb(&recipient_tag)
        }
        .expect("attempted to retransmit a packet to an unknown recipient - we shouldn't have sent the original packet in the first place!");

        if let Some(reply_surb) = maybe_reply_surb {
            match self
                .message_handler
                .try_prepare_single_reply_chunk_for_sending(reply_surb, ack_ref.fragment_data())
                .await
            {
                Ok(prepared) => {
                    // drop the ack ref so that controller would not panic on `UpdateTimer` if that task
                    // got to handle the action before this function terminated (which is very much
                    // possible if `forward_messages` takes a while)
                    drop(ack_ref);

                    self.message_handler
                        .update_ack_delay(prepared.fragment_identifier, prepared.total_delay);
                    self.message_handler
                        .forward_messages(vec![prepared.into()], TransmissionLane::Retransmission)
                        .await;
                }
                Err(err) => {
                    let err = err.return_unused_surbs(
                        self.full_reply_storage.surbs_storage_ref(),
                        &recipient_tag,
                    );
                    warn!("failed to prepare message for retransmission - {err}");
                    // we buffer that packet and to try another day
                    self.buffer_pending_ack(recipient_tag, ack_ref, timed_out_ack);

                    if self.should_request_more_surbs(&recipient_tag) {
                        self.request_reply_surbs_for_queue_clearing(recipient_tag)
                            .await;
                    }
                }
            };
        } else {
            self.buffer_pending_ack(recipient_tag, ack_ref, timed_out_ack);

            if self.should_request_more_surbs(&recipient_tag) {
                self.request_reply_surbs_for_queue_clearing(recipient_tag)
                    .await;
            }
        }
    }

    // to be honest this doesn't make a lot of sense in the context of `connection_id`,
    // it should really be asked per tag
    fn handle_lane_queue_length(
        &self,
        connection_id: ConnectionId,
        response_channel: oneshot::Sender<usize>,
    ) {
        // TODO: if we ever have duplicate ids for different senders, it means our rng is super weak
        // thus I don't think we have to worry about it?
        let lane = TransmissionLane::ConnectionId(connection_id);
        for buf in self.pending_replies.values() {
            if let Some(length) = buf.lane_length(&lane) {
                if response_channel.send(length).is_err() {
                    error!("the requester for lane queue length has dropped the response channel!")
                }
                return;
            }
        }
        // make sure that if we didn't find that lane, we reply with 0
        if response_channel.send(0).is_err() {
            error!("the requester for lane queue length has dropped the response channel!")
        }
    }

    async fn handle_request(&mut self, request: ReplyControllerMessage) {
        match request {
            ReplyControllerMessage::RetransmitReply {
                recipient,
                timed_out_ack,
                extra_surb_request,
            } => {
                self.handle_reply_retransmission(recipient, timed_out_ack, extra_surb_request)
                    .await
            }
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
            ReplyControllerMessage::LaneQueueLength {
                connection_id,
                response_channel,
            } => self.handle_lane_queue_length(connection_id, response_channel),
            ReplyControllerMessage::AdditionalSurbsRequest { recipient, amount } => {
                self.handle_surb_request(*recipient, amount).await
            }
        }
    }

    // TODO: modify this method to more accurately determine the amount of surbs it needs to request
    // it should take into consideration the average latency, sending rate and queue size.
    // it should request as many surbs as it takes to saturate its sending rate before next batch arrives
    async fn request_reply_surbs_for_queue_clearing(&mut self, target: AnonymousSenderTag) {
        trace!("requesting surbs for queues clearing");

        let pending_queue_size = self
            .pending_replies
            .get(&target)
            .map(|pending_queue| pending_queue.total_size())
            .unwrap_or_default();

        let retransmission_queue = self
            .pending_retransmissions
            .get(&target)
            .map(|pending_queue| pending_queue.len())
            .unwrap_or_default();

        let total_queue = (pending_queue_size + retransmission_queue) as u32;

        if total_queue == 0 {
            trace!("the pending queues for {:?} are already empty", target);
            return;
        }

        let request_size = min(
            self.config.reply_surbs.maximum_reply_surb_request_size,
            max(
                total_queue,
                self.config.reply_surbs.minimum_reply_surb_request_size,
            ),
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
        let mut to_remove = Vec::new();

        let now = OffsetDateTime::now_utc();
        for (pending_reply_target, vals) in &self.pending_replies {
            if vals.is_empty() {
                continue;
            }

            let Some(last_received) = self
                .full_reply_storage
                .surbs_storage_ref()
                .surbs_last_received_at(pending_reply_target)
            else {
                error!("we have {} pending replies for {pending_reply_target}, but we somehow never received any reply surbs from them!", vals.total_size());
                to_remove.push(*pending_reply_target);
                continue;
            };

            // this should never ever happen (famous last words, eh?), but in case it DOES happen eventually
            // purge that malformed data
            let Ok(last_received_time) = OffsetDateTime::from_unix_timestamp(last_received) else {
                error!("somehow our stored timestamp ({last_received}) for surbs from {pending_reply_target} is corrupted!. Going to remove all the associated entries");
                to_remove.push(*pending_reply_target);
                continue;
            };

            let diff = now - last_received_time;
            let max_rerequest_wait = self
                .config
                .reply_surbs
                .maximum_reply_surb_rerequest_waiting_period;
            let max_drop_wait = self
                .config
                .reply_surbs
                .maximum_reply_surb_drop_waiting_period;

            if diff > max_rerequest_wait {
                if diff > max_drop_wait {
                    to_remove.push(*pending_reply_target)
                } else {
                    debug!("We haven't received any surbs in {:?} from {pending_reply_target}. Going to explicitly ask for more", diff);
                    to_request.push(*pending_reply_target);
                }
            }
        }

        for pending_reply_target in to_request {
            self.request_reply_surbs_for_queue_clearing(pending_reply_target)
                .await;
            self.full_reply_storage
                .surbs_storage_ref()
                .reset_pending_reception(&pending_reply_target)
        }
        for to_remove in to_remove {
            self.pending_replies.remove(&to_remove);
        }
    }

    async fn invalidate_old_data(&self) {
        let now = OffsetDateTime::now_utc();

        let mut to_remove_surbs = Vec::new();
        let mut to_remove_keys = Vec::new();
        for map_ref in self.full_reply_storage.surbs_storage_ref().as_raw_iter() {
            let (sender, received) = map_ref.pair();
            // TODO: handle the following edge case:
            // there's a malicious client sending us exactly one reply surb just before we should have invalidated
            // the data thus making us keep everything in memory
            // possible solution: keep timestamp PER reply surb (but that seems like an overkill)
            // but I doubt this is ever going to be a problem...
            // ...
            // However, if you're reading this message, it probably became a legit problem,
            // so I guess add timestamp per surb then? chop-chop.

            let last_received = received.surbs_last_received_at();
            // this should never ever happen (famous last words, eh?), but in case it DOES happen eventually
            // purge that malformed data
            let Ok(last_received_time) = OffsetDateTime::from_unix_timestamp(last_received) else {
                error!("somehow our stored timestamp ({last_received}) for surbs from {sender} is corrupted!. Going to remove all the associated entries");
                to_remove_surbs.push(*sender);
                continue;
            };
            let diff = now - last_received_time;

            if diff > self.config.reply_surbs.maximum_reply_surb_age {
                info!("it's been {diff:?} since we last received any reply surb from {sender}. Going to remove all stored entries...");

                to_remove_surbs.push(*sender);
            }
        }

        for map_ref in self.full_reply_storage.key_storage_ref().as_raw_iter() {
            let (digest, reply_key) = map_ref.pair();

            // this should never ever happen (famous last words, eh?), but in case it DOES happen eventually
            // purge that malformed data
            let Ok(sent_at) = OffsetDateTime::from_unix_timestamp(reply_key.sent_at_timestamp)
            else {
                error!("somehow our stored timestamp ({}) for one of our reply key is corrupted!. Going to remove all the entry", reply_key.sent_at_timestamp);
                to_remove_keys.push(*digest);
                continue;
            };

            let diff = now - sent_at;

            if diff > self.config.reply_surbs.maximum_reply_key_age {
                debug!("it's been {diff:?} since we created this reply key. it's probably never going to get used, so we're going to purge it...");
                to_remove_keys.push(*digest);
            }
        }

        for to_remove in to_remove_surbs {
            self.full_reply_storage
                .surbs_storage_ref()
                .remove(&to_remove);
        }

        for to_remove in to_remove_keys {
            self.full_reply_storage.key_storage().remove(to_remove)
        }
    }

    // #[cfg(not(target_arch = "wasm32"))]
    // async fn log_status(&self) {
    //     todo!()
    // }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        debug!("Started ReplyController with graceful shutdown support");

        let polling_rate = Duration::from_secs(5);
        let mut stale_inspection = new_interval_stream(polling_rate);

        // this is in the order of hours/days so we don't have to poll it that often
        let polling_rate =
            Duration::from_secs(self.config.reply_surbs.maximum_reply_surb_age.as_secs() / 10);
        let mut invalidation_inspection = new_interval_stream(polling_rate);

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv_with_delay() => {
                    log::trace!("ReplyController: Received shutdown");
                },
                req = self.request_receiver.next() => match req {
                    Some(req) => self.handle_request(req).await,
                    None => {
                        log::trace!("ReplyController: Stopping since channel closed");
                        break;
                    }
                },
                _ = stale_inspection.next() => {
                    self.inspect_stale_entries().await
                },
                _ = invalidation_inspection.next() => {
                    self.invalidate_old_data().await
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("ReplyController: Exiting");
    }
}
