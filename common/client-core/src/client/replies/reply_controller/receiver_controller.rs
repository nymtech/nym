// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::real_messages_control::acknowledgement_control::PendingAcknowledgement;
use crate::client::real_messages_control::message_handler::{
    FragmentWithMaxRetransmissions, MessageHandler, PreparationError,
};
use crate::client::replies::reply_controller::key_rotation_helpers::SurbRefreshState;
use crate::client::replies::reply_controller::Config;
use crate::client::topology_control::TopologyAccessor;
use crate::client::transmission_buffer::TransmissionBuffer;
use futures::channel::oneshot;
use nym_client_core_surb_storage::{ReceivedReplySurb, ReceivedReplySurbsMap};
use nym_crypto::aes::cipher::crypto_common::rand_core::CryptoRng;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::anonymous_replies::ReplySurbWithKeyRotation;
use nym_sphinx::chunking::fragment::FragmentIdentifier;
use nym_task::connections::{ConnectionId, TransmissionLane};
use nym_topology::NymTopologyMetadata;
use rand::Rng;
use std::cmp::{max, min};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use std::sync::{Arc, Weak};
use time::OffsetDateTime;
use tracing::{debug, error, info, trace, warn};

struct SenderData {
    current_clear_rerequest_counter: usize,
    pending_replies: TransmissionBuffer<FragmentWithMaxRetransmissions>,
    pending_retransmissions: BTreeMap<FragmentIdentifier, Weak<PendingAcknowledgement>>,
    last_request_failure: OffsetDateTime,
}

impl Default for SenderData {
    fn default() -> Self {
        SenderData {
            current_clear_rerequest_counter: 0,
            pending_replies: Default::default(),
            pending_retransmissions: Default::default(),
            last_request_failure: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

impl SenderData {
    fn total_pending(&self) -> usize {
        let pending_replies = self.pending_replies.total_size();
        let pending_retransmissions = self.pending_retransmissions.len();
        let total_pending = pending_retransmissions + pending_replies;

        debug!("total queue size: {total_pending} = pending data {pending_replies} + pending retransmission {pending_retransmissions}");

        total_pending
    }

    pub(crate) fn increment_current_clear_rerequest_counter(&mut self) {
        self.current_clear_rerequest_counter += 1;
    }

    pub(crate) fn reset_current_clear_rerequest_counter(&mut self) {
        self.current_clear_rerequest_counter = 0;
    }

    pub(crate) fn reset_last_request_failure(&mut self, now: OffsetDateTime) -> OffsetDateTime {
        mem::replace(&mut self.last_request_failure, now)
    }
}

/// Reply controller responsible for controlling receiver-related part
/// of replies, such as requesting additional reply SURBs
pub struct ReceiverReplyController<R> {
    config: Config,

    surb_refresh_state: SurbRefreshState,
    topology_access: TopologyAccessor,

    surb_senders: HashMap<AnonymousSenderTag, SenderData>,
    unavailable: HashMap<AnonymousSenderTag, OffsetDateTime>,
    surbs_storage: ReceivedReplySurbsMap,

    // TODO: incorporate that field at some point
    // and use binomial distribution to determine the expected required number
    // of surbs required to send the message through
    // expected_reliability: f32,
    message_handler: MessageHandler<R>,
}

impl<R> ReceiverReplyController<R>
where
    R: CryptoRng + Rng,
{
    pub(crate) fn new(
        config: Config,
        storage: ReceivedReplySurbsMap,
        message_handler: MessageHandler<R>,
    ) -> Self {
        let topology_access = message_handler.topology_access_handle().clone();

        ReceiverReplyController {
            config,
            surb_refresh_state: SurbRefreshState::WaitingForNextRotation {
                last_known: config
                    .key_rotation
                    .expected_current_key_rotation_id(OffsetDateTime::now_utc()),
            },
            topology_access,
            surb_senders: Default::default(),
            unavailable: Default::default(),
            surbs_storage: storage,
            message_handler,
        }
    }

    fn get_or_create_surb_sender(&mut self, tag: &AnonymousSenderTag) -> &mut SenderData {
        self.surb_senders.entry(*tag).or_default()
    }

    async fn current_topology_metadata(&self) -> Option<NymTopologyMetadata> {
        self.topology_access.current_metadata().await
    }

    fn insert_pending_replies<I: IntoIterator<Item = FragmentWithMaxRetransmissions>>(
        &mut self,
        recipient: &AnonymousSenderTag,
        fragments: I,
        lane: TransmissionLane,
    ) {
        trace!("buffering pending replies for {recipient}");
        self.surb_senders
            .entry(*recipient)
            .or_default()
            .pending_replies
            .store(&lane, fragments)
    }

    fn re_insert_pending_replies(
        &mut self,
        recipient: &AnonymousSenderTag,
        fragments: Vec<(TransmissionLane, FragmentWithMaxRetransmissions)>,
    ) {
        trace!("re-inserting pending replies for {recipient}");
        // the buffer should ALWAYS exist at this point, if it doesn't, it's a bug...
        self.surb_senders
            .entry(*recipient)
            .or_default()
            .pending_replies
            .store_multiple(fragments)
    }

    fn re_insert_pending_retransmission(
        &mut self,
        recipient: &AnonymousSenderTag,
        data: Vec<Arc<PendingAcknowledgement>>,
    ) {
        trace!("re-inserting pending retransmissions for {recipient}");
        // the underlying entry MUST exist as we've just got data from there
        // and we hold a mut reference
        let map_entry = &mut self
            .surb_senders
            .get_mut(recipient)
            .expect("our pending retransmission entry is somehow gone!")
            .pending_retransmissions;

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
        trace!("checking if we should request more surbs from {target}");

        let total_queue = self
            .surb_senders
            .get(target)
            .map(|pending| pending.total_pending())
            .unwrap_or_default();

        // only consider 'fresh' surbs
        let available_surbs = self.surbs_storage.available_fresh_surbs(target);
        let pending_surbs = self.surbs_storage.pending_reception(target) as usize;
        let min_surbs_threshold = self.surbs_storage.min_surb_threshold();
        let max_surbs_threshold = self.surbs_storage.max_surb_threshold();
        let min_surbs_threshold_buffer =
            self.config.reply_surbs.minimum_reply_surb_threshold_buffer;

        // After clearing the queue, we want to have at least `min_surbs_threshold` surbs available
        // and reserved for requesting additional surbs, and in addition to that we also want to
        // have `min_surbs_threshold_buffer` surbs available proactively.
        let target_surbs_after_clearing_queue = min_surbs_threshold + min_surbs_threshold_buffer;

        // Check if we have enough surbs to handle the total queue and maintain minimum thresholds
        let total_required_surbs = total_queue + target_surbs_after_clearing_queue;
        let total_available_surbs = pending_surbs + available_surbs;

        debug!("available surbs: {available_surbs} pending surbs: {pending_surbs} threshold range: {min_surbs_threshold}..+{min_surbs_threshold_buffer}..{max_surbs_threshold}");

        // We should request more surbs if:
        // 1. We haven't hit the maximum surb threshold, and
        // 2. We don't have enough surbs to handle the queue plus minimum thresholds
        let is_below_max_threshold = total_available_surbs < max_surbs_threshold;
        let is_below_required_surbs = total_available_surbs < total_required_surbs;

        is_below_max_threshold && is_below_required_surbs
    }

    pub(crate) async fn handle_send_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: Vec<u8>,
        lane: TransmissionLane,
        max_retransmissions: Option<u32>,
    ) {
        if !self.surbs_storage.contains_surbs_for(&recipient_tag) {
            if self
                .unavailable
                .insert(recipient_tag, OffsetDateTime::now_utc())
                .is_none()
            {
                // don't report it every single time
                warn!("received reply request for {recipient_tag} but we don't have any surbs stored for that recipient!");
            } else {
                trace!("received reply request for {recipient_tag} but we don't have any surbs stored for that recipient!");
            }
            return;
        }

        trace!("handling reply to {recipient_tag}");
        let mut fragments = self.message_handler.split_reply_message(data);
        let total_size = fragments.len();
        trace!("This reply requires {total_size} SURBs");

        // for the purposes of sending reply, do allow using possibly stale entries
        let available_surbs = self.surbs_storage.available_surbs(&recipient_tag);
        let min_surbs_threshold = self.surbs_storage.min_surb_threshold();

        let max_to_send = if available_surbs > min_surbs_threshold {
            min(fragments.len(), available_surbs - min_surbs_threshold)
        } else {
            0
        };

        if max_to_send > 0 {
            let (surbs, surbs_left) = self
                .surbs_storage
                .get_reply_surbs(&recipient_tag, max_to_send);

            debug!(
                "retrieved {} reply surbs. {surbs_left} surbs remaining in storage",
                surbs.as_ref().map(|s| s.len()).unwrap_or_default()
            );
            if let Some(reply_surbs) = surbs {
                let to_send = fragments
                    .drain(..reply_surbs.len())
                    .map(|f| FragmentWithMaxRetransmissions {
                        fragment: f,
                        max_retransmissions,
                    })
                    .collect::<Vec<_>>();

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
                    let err = err.return_unused_surbs(&self.surbs_storage, &recipient_tag);
                    warn!("failed to send reply to {recipient_tag}: {err}");
                    info!(
                        "buffering {no_fragments} fragments for {recipient_tag}",
                        no_fragments = to_send.len()
                    );
                    self.insert_pending_replies(&recipient_tag, to_send, lane);
                }
            }
        }

        // if there's leftover data we didn't send because we didn't have enough (or any) surbs - buffer it
        if !fragments.is_empty() {
            // Ideally we should have enough surbs above the minimum threshold to handle sending
            // new replies without having to first request more surbs. That's why I'd like to log
            // these cases as they might indicate a problem with the surb management.
            debug!(
                "buffering {no_fragments} fragments for {recipient_tag}",
                no_fragments = fragments.len()
            );
            let fragments: Vec<_> = fragments
                .into_iter()
                .map(|fragment| FragmentWithMaxRetransmissions {
                    fragment,
                    max_retransmissions,
                })
                .collect();
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
        debug!("requesting {amount} additional reply surbs for {target}");
        let (reply_surb, _) = self
            .surbs_storage
            .get_reply_surb_ignoring_threshold(&target);

        let reply_surb = reply_surb.ok_or(PreparationError::NotEnoughSurbs {
            available: 0,
            required: 1,
        })?;

        if let Err(err) = self
            .message_handler
            .try_request_additional_reply_surbs(target, reply_surb, amount)
            .await
        {
            let err = err.return_unused_surbs(&self.surbs_storage, &target);
            warn!("failed to request additional surbs from {target}: {err}",);
            return Err(err);
        } else {
            self.surbs_storage
                .increment_pending_reception(&target, amount);
        }

        Ok(())
    }

    async fn try_clear_pending_retransmission(&mut self, target: AnonymousSenderTag) {
        trace!("trying to clear pending retransmission queue");
        let available_surbs = self.surbs_storage.available_surbs(&target);
        let min_surbs_threshold = self.surbs_storage.min_surb_threshold();

        let max_to_clear = if available_surbs > min_surbs_threshold {
            available_surbs - min_surbs_threshold
        } else {
            trace!("we don't have enough surbs for retransmission queue clearing...");
            return;
        };
        trace!("we can clear up to {max_to_clear} entries");

        let Some(pending) = self.surb_senders.get_mut(&target) else {
            trace!("no pending entry for {target}!");
            return;
        };

        let mut to_take = Vec::new();

        while to_take.len() < max_to_clear {
            if let Some((_, data)) = pending.pending_retransmissions.pop_first() {
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

        let (surbs_for_reply, _) = self.surbs_storage.get_reply_surbs(&target, to_take.len());

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
                let err = err.return_unused_surbs(&self.surbs_storage, &target);
                self.re_insert_pending_retransmission(&target, to_take);

                warn!("failed to clear pending retransmission queue for {target}: {err}",);
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
    ) -> Option<Vec<(TransmissionLane, FragmentWithMaxRetransmissions)>> {
        // if possible, pop all pending replies, if not, pop only entries for which we'd have a reply surb
        let pending = self.surb_senders.get_mut(from)?;
        let total = pending.pending_replies.total_size();
        trace!("pending queue has {total} elements");
        if total == 0 {
            return None;
        }
        pending
            .pending_replies
            .pop_at_most_n_next_messages_at_random(amount)
    }

    async fn try_clear_pending_queue(&mut self, target: AnonymousSenderTag) {
        trace!("trying to clear pending queue");
        let available_surbs = self.surbs_storage.available_surbs(&target);
        let min_surbs_threshold = self.surbs_storage.min_surb_threshold();

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
                .surbs_storage
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
                let err = err.return_unused_surbs(&self.surbs_storage, &target);
                self.re_insert_pending_replies(&target, to_send);
                warn!("failed to clear pending queue for {target}: {err}");
            }
        } else {
            trace!("the pending queue is empty");
        }
    }

    fn reset_rerequest_counter(&mut self, from: &AnonymousSenderTag) {
        if let Some(pending) = self.surb_senders.get_mut(from) {
            pending.reset_current_clear_rerequest_counter()
        }
    }

    pub(crate) async fn handle_received_surbs(
        &mut self,
        from: AnonymousSenderTag,
        reply_surbs: Vec<ReplySurbWithKeyRotation>,
        from_surb_request: bool,
    ) {
        trace!("handling received surbs");

        // clear the requesting flag since we should have been asking for surbs
        if from_surb_request {
            self.surbs_storage
                .decrement_pending_reception(&from, reply_surbs.len() as u32);
        }

        // store received surbs
        self.surbs_storage.insert_fresh_surbs(&from, reply_surbs);

        // reset, if applicable, request counter
        self.reset_rerequest_counter(&from);

        // use as many as we can for clearing pending retransmission queue
        self.try_clear_pending_retransmission(from).await;

        // use as many as we can for clearing pending 'normal' queue
        self.try_clear_pending_queue(from).await;

        // if we have to, request more
        if self.should_request_more_surbs(&from) {
            self.request_reply_surbs_for_queue_clearing(from).await;
        }
    }
    fn buffer_pending_ack(
        &mut self,
        recipient: AnonymousSenderTag,
        ack_ref: Arc<PendingAcknowledgement>,
        weak_ack_ref: Weak<PendingAcknowledgement>,
    ) {
        let frag_id = ack_ref.inner_fragment_identifier();

        let pending = self.surb_senders.entry(recipient).or_default();
        if let Entry::Vacant(e) = pending.pending_retransmissions.entry(frag_id) {
            e.insert(weak_ack_ref);
        } else {
            warn!(
                "we're already trying to retransmit {frag_id}. We must be really behind in surbs!"
            );
        }
    }

    pub(crate) async fn handle_reply_retransmission(
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
            self.surbs_storage
                .get_reply_surb_ignoring_threshold(&recipient_tag)
        } else {
            self.surbs_storage.get_reply_surb(&recipient_tag)
        };

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
                    let err = err.return_unused_surbs(&self.surbs_storage, &recipient_tag);
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
    pub(crate) fn handle_lane_queue_length(
        &self,
        connection_id: ConnectionId,
        response_channel: oneshot::Sender<usize>,
    ) {
        // TODO: if we ever have duplicate ids for different senders, it means our rng is super weak
        // thus I don't think we have to worry about it?
        let lane = TransmissionLane::ConnectionId(connection_id);
        for buf in self.surb_senders.values().map(|p| &p.pending_replies) {
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

    // TODO: modify this method to more accurately determine the amount of surbs it needs to request
    // it should take into consideration the average latency, sending rate and queue size.
    // it should request as many surbs as it takes to saturate its sending rate before next batch arrives
    async fn request_reply_surbs_for_queue_clearing(&mut self, target: AnonymousSenderTag) {
        trace!("requesting surbs for queue clearing");

        let total_queue = self
            .surb_senders
            .get(&target)
            .map(|pending| pending.total_pending() as u32)
            .unwrap_or_default();

        let min_surbs_buffer = self.config.reply_surbs.minimum_reply_surb_threshold_buffer as u32;

        // To proactively request additional surbs, we aim to have a buffer of extra surbs in our
        // storage.
        let total_queue_with_buffer = total_queue + min_surbs_buffer;

        let request_size = min(
            self.config.reply_surbs.maximum_reply_surb_request_size,
            max(
                total_queue_with_buffer,
                self.config.reply_surbs.minimum_reply_surb_request_size,
            ),
        );

        if let Err(err) = self
            .request_additional_reply_surbs(target, request_size)
            .await
        {
            let now = OffsetDateTime::now_utc();
            let sender_info = self.get_or_create_surb_sender(&target);
            let last_failure = sender_info.reset_last_request_failure(now);

            // only log at higher level if it's the first time this error has occurred in a while
            if now - last_failure > time::Duration::seconds(30) {
                warn!("failed to request more surbs to clear pending queue of size {total_queue} (attempted to request: {request_size}): {err}")
            } else {
                debug!("failed to request more surbs to clear pending queue of size {total_queue} (attempted to request: {request_size}): {err}")
            }
        }
    }

    pub(crate) async fn inspect_stale_pending_data(&mut self) {
        let mut to_request = Vec::new();
        let mut to_remove = Vec::new();

        let now = OffsetDateTime::now_utc();
        for (pending_reply_target, vals) in self.surb_senders.iter_mut() {
            // for now recreate old behaviour
            let retransmission_buf = &vals.pending_replies;

            if retransmission_buf.is_empty() {
                continue;
            }

            let Some(last_received_time) = self
                .surbs_storage
                .surbs_last_received_at(pending_reply_target)
            else {
                error!("we have {} pending replies for {pending_reply_target}, but we somehow never received any reply surbs from them!", retransmission_buf.total_size());
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
            let max_rerequests = self.config.reply_surbs.maximum_reply_surbs_rerequests;

            // if we have already requested extra surbs because of the stale entry,
            // don't do it again (otherwise we'll get stuck in a constant cycle of requesting more surbs
            // if client is offline)
            if vals.current_clear_rerequest_counter > max_rerequests {
                to_remove.push(*pending_reply_target);
                debug!("we have reached the maximum threshold of attempting to request surbs from {pending_reply_target}. dropping the sender");
                continue;
            }

            if diff > max_rerequest_wait {
                if diff > max_drop_wait {
                    to_remove.push(*pending_reply_target)
                } else {
                    debug!("We haven't received any surbs in {} from {pending_reply_target}. Going to explicitly ask for more", humantime::format_duration(diff.unsigned_abs()));
                    vals.increment_current_clear_rerequest_counter();
                    to_request.push(*pending_reply_target);
                }
            }
        }

        for pending_reply_target in to_request {
            self.request_reply_surbs_for_queue_clearing(pending_reply_target)
                .await;
            self.surbs_storage
                .reset_pending_reception(&pending_reply_target)
        }
        for to_remove in to_remove {
            // TODO: in the 'old' version we just removed pending messages,
            // not retransmissions, but I think those should follow the same logic.
            // if something breaks because of that. I guess here is your explanation, future reader
            self.surb_senders.remove(&to_remove);
        }
    }

    pub(crate) async fn check_surb_refresh(&mut self) {
        let Some(current_rotation_id) = self.topology_access.current_key_rotation_id().await else {
            warn!("failed to retrieve current key rotation id from the network topology");
            return;
        };

        if let SurbRefreshState::WaitingForNextRotation { last_known } = self.surb_refresh_state {
            if last_known == current_rotation_id {
                trace!("no changes in key rotation id");
            } else {
                // key rotation actually changed and given the polling rate (1/8th epoch) we should have plenty
                // of time to perform the upgrade.
                // but wait for one more call before doing this so that the clients could also resync
                // their topologies and discover new rotation
                self.surb_refresh_state = SurbRefreshState::ScheduledForNextInvocation;
            }
            return;
        }

        // here we are in `SurbRefreshState::ScheduledForNextInvocation` state

        let mut marked_as_stale = HashMap::new();

        // 1. mark all existing surbs we have as possibly stale
        for mut map_entry in self.surbs_storage.as_raw_iter_mut() {
            let (sender, received) = map_entry.pair_mut();
            let num_downgraded = received.downgrade_freshness();
            trace!("{sender}: {num_downgraded} downgraded");
            if num_downgraded != 0 {
                marked_as_stale.insert(*sender, num_downgraded);
            }
        }

        // 2. attempt to re-request the equivalent number of fresh surbs
        // TODO PROBLEM: if our request gets lost, we might be in trouble...
        // we need some sort of retry mechanism
        for (sender, num_to_request) in marked_as_stale {
            if self
                .request_additional_reply_surbs(sender, num_to_request as u32)
                .await
                .is_err()
            {
                warn!("surb refresh request failed")
            }
        }

        self.surb_refresh_state = SurbRefreshState::WaitingForNextRotation {
            last_known: current_rotation_id,
        };
    }

    pub(crate) async fn inspect_and_clear_stale_data(&mut self, now: OffsetDateTime) {
        // technically we don't know if epoch is stuck, but we're flying in blind here,
        // so we have to assume the worst and not purge anything depending on proper epoch progression
        let is_epoch_stuck = self
            .current_topology_metadata()
            .await
            .map(|m| self.config.key_rotation.epoch_stuck(m))
            .unwrap_or(false);

        // expected time of when the CURRENT key rotation has begun
        let expected_current_key_rotation_start = self
            .config
            .key_rotation
            .expected_current_key_rotation_start(now);

        // expected ID of the CURRENT key rotation
        let expected_current_key_rotation = self
            .config
            .key_rotation
            .expected_current_key_rotation_id(now);

        // time of the start of one epoch BEFORE the CURRENT rotation has begun
        // this indicates the starting time of when packets with the current keys might have been constructed
        let prior_epoch_start =
            expected_current_key_rotation_start - self.config.key_rotation.epoch_duration;

        // time of the start of one epoch AFTER the current rotation has begun
        // this indicates the end of transition period and any packets constructed with keys different
        // from the current one are definitely invalid
        let following_epoch_start =
            expected_current_key_rotation_start + self.config.key_rotation.epoch_duration;

        // define a closure for validating individual surbs
        // (we have to run it twice for different piles)
        let basic_surb_retention_logic = |received_surb: &ReceivedReplySurb| {
            if is_epoch_stuck {
                let diff = now - received_surb.received_at();
                return diff < self.config.key_rotation.rotation_lifetime();
            }

            if received_surb.received_at() < prior_epoch_start {
                // it's definitely from previous rotation
                return false;
            }
            let surb_rotation = received_surb.key_rotation();

            if surb_rotation.is_unknown() {
                // can't do anything, so just retain it
                return true;
            }

            // TODO: will this backfire during transition period where we need surbs to refresh surbs
            // and we failed to send a request?
            if surb_rotation.is_even() && expected_current_key_rotation % 2 == 1 {
                return false;
            }

            if surb_rotation.is_odd() && expected_current_key_rotation % 2 == 0 {
                return false;
            }

            true
        };

        // 1. purge full old clients data (this applies to RECEIVER)
        self.surbs_storage.retain(|_, received| {
            if is_epoch_stuck {
                // if epoch is stuck, we can't do much (because we don't know for certain if rotation has advanced)
                // apart from the basic check of surbs being received more than maximum lifetime of a rotation
                // because at that point we know they must be invalid
                let diff = now - received.surbs_last_received_at();
                return diff < self.config.key_rotation.rotation_lifetime();
            }

            // if surbs were received more than 1h before the start of the current rotation,
            // they're DEFINITELY invalid.
            // if it was up until 1h AFTER the start of the current rotation they MIGHT be valid -
            // we don't know for sure, unless the client explicitly attached rotation information
            // (which only applies to more recent versions of clients so we can't 100% rely on that)
            if received.surbs_last_received_at() < prior_epoch_start {
                return false;
            }

            // 1.1. check individual surbs (same basic logic applies)
            received.retain_fresh_surbs(&basic_surb_retention_logic);

            // 1.2. check the possibly stale entries
            // 1.2.1. check if we're beyond the key rotation transition period,
            // if so those surbs are definitely unusable
            if now > following_epoch_start {
                received.drop_possibly_stale_surbs();
            }

            // 1.2.2. otherwise continue with the same logic as the fresh ones
            received.retain_possibly_stale_surbs(&basic_surb_retention_logic);

            // no surbs left, we're not expecting any AND we haven't received anything in a while
            // (i.e. sender probably abandoned us)
            let max_drop_wait = self
                .config
                .reply_surbs
                .maximum_reply_surb_drop_waiting_period;
            let last_received = received.surbs_last_received_at();

            let possibly_abandoned = last_received + max_drop_wait < now;
            if received.is_empty() && received.pending_reception() == 0 && possibly_abandoned {
                return false;
            }

            true
        });

        // 1.3 inspect old unavailable receivers to clear any stale data
        self.unavailable
            .retain(|_, last_reported| now - *last_reported < time::Duration::seconds(30));
    }
}
