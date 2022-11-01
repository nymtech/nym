// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::inbound_messages::TransmissionLane;
use crate::client::mix_traffic::BatchMixMessageSender;
use crate::client::real_messages_control::acknowledgement_control::SentPacketNotificationSender;
use crate::client::topology_control::TopologyAccessor;
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::*;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::chunking::fragment::FragmentIdentifier;
use nymsphinx::cover::generate_loop_cover_packet;
use nymsphinx::forwarding::packet::MixPacket;
use nymsphinx::params::PacketSize;
use nymsphinx::utils::sample_poisson_duration;
use rand::seq::SliceRandom;
use rand::{CryptoRng, Rng};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

#[cfg(target_arch = "wasm32")]
use wasm_timer;

// The minimum time between increasing the average delay between packets. If we hit the ceiling in
// the available buffer space we want to take somewhat swift action, but we still need to give a
// short time to give the channel a chance reduce pressure.
const INCREASE_DELAY_MIN_CHANGE_INTERVAL_SECS: u64 = 1;
// The minimum time between decreasing the average delay between packets. We don't want to change
// to quickly to keep things somewhat stable. Also there are buffers downstreams meaning we need to
// wait a little to see the effect before we decrease further.
const DECREASE_DELAY_MIN_CHANGE_INTERVAL_SECS: u64 = 30;
// If we enough time passes without any sign of backpressure in the channel, we can consider
// lowering the average delay. The goal is to keep somewhat stable, rather than maxing out
// bandwidth at all times.
const ACCEPTABLE_TIME_WITHOUT_BACKPRESSURE_SECS: u64 = 30;
// The maximum multiplier we apply to the base average Poisson delay.
const MAX_DELAY_MULTIPLIER: u32 = 6;
// The minium multiplier we apply to the base average Poisson delay.
const MIN_DELAY_MULTIPLIER: u32 = 1;

/// Configurable parameters of the `OutQueueControl`
pub(crate) struct Config {
    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Average delay a data packet is going to get delay at a single mixnode.
    average_packet_delay: Duration,

    /// Average delay between sending subsequent packets.
    average_message_sending_delay: Duration,

    /// Controls whether the stream constantly produces packets according to the predefined
    /// poisson distribution.
    disable_poisson_packet_distribution: bool,

    /// Predefined packet size used for the loop cover messages.
    cover_packet_size: PacketSize,
}

impl Config {
    pub(crate) fn new(
        average_ack_delay: Duration,
        average_packet_delay: Duration,
        average_message_sending_delay: Duration,
        disable_poisson_packet_distribution: bool,
    ) -> Self {
        Config {
            average_ack_delay,
            average_packet_delay,
            average_message_sending_delay,
            disable_poisson_packet_distribution,
            cover_packet_size: Default::default(),
        }
    }

    pub fn with_custom_cover_packet_size(mut self, packet_size: PacketSize) -> Self {
        self.cover_packet_size = packet_size;
        self
    }
}

struct SendingDelayController {
    /// Multiply the average sending delay.
    /// This is normally set to unity, but if we detect backpressure we increase this
    /// multiplier. We use discrete steps.
    current_multiplier: u32,

    /// Maximum delay multiplier
    upper_bound: u32,

    /// Minimum delay multiplier
    lower_bound: u32,

    /// To make sure we don't change the multiplier to fast, we limit a change to some duration
    #[cfg(not(target_arch = "wasm32"))]
    time_when_changed: time::Instant,

    #[cfg(target_arch = "wasm32")]
    time_when_changed: wasm_timer::Instant,

    /// If we have a long enough time without any backpressure detected we try reducing the sending
    /// delay multiplier
    #[cfg(not(target_arch = "wasm32"))]
    time_when_backpressure_detected: time::Instant,

    #[cfg(target_arch = "wasm32")]
    time_when_backpressure_detected: wasm_timer::Instant,
}

#[cfg(not(target_arch = "wasm32"))]
fn get_time_now() -> time::Instant {
    time::Instant::now()
}

#[cfg(target_arch = "wasm32")]
fn get_time_now() -> wasm_timer::Instant {
    wasm_timer::Instant::now()
}

impl SendingDelayController {
    fn new(lower_bound: u32, upper_bound: u32) -> Self {
        assert!(lower_bound <= upper_bound);
        let now = get_time_now();
        SendingDelayController {
            current_multiplier: MIN_DELAY_MULTIPLIER,
            upper_bound,
            lower_bound,
            time_when_changed: now,
            time_when_backpressure_detected: now,
        }
    }

    fn current_multiplier(&self) -> u32 {
        self.current_multiplier
    }

    fn increase_delay_multiplier(&mut self) {
        if self.current_multiplier < self.upper_bound {
            self.current_multiplier =
                (self.current_multiplier + 1).clamp(self.lower_bound, self.upper_bound);
            self.time_when_changed = get_time_now();
            log::debug!(
                "Increasing sending delay multiplier to: {}",
                self.current_multiplier
            );
        } else {
            log::warn!("Trying to increase delay multipler higher than allowed");
        }
    }

    fn decrease_delay_multiplier(&mut self) {
        if self.current_multiplier > self.lower_bound {
            self.current_multiplier =
                (self.current_multiplier - 1).clamp(self.lower_bound, self.upper_bound);
            self.time_when_changed = get_time_now();
            log::debug!(
                "Decreasing sending delay multiplier to: {}",
                self.current_multiplier
            );
        }
    }

    fn record_backpressure_detected(&mut self) {
        self.time_when_backpressure_detected = get_time_now();
    }

    fn not_increased_delay_recently(&self) -> bool {
        get_time_now()
            > self.time_when_changed + Duration::from_secs(INCREASE_DELAY_MIN_CHANGE_INTERVAL_SECS)
    }

    fn is_sending_reliable(&self) -> bool {
        let now = get_time_now();
        let delay_change_interval = Duration::from_secs(DECREASE_DELAY_MIN_CHANGE_INTERVAL_SECS);
        let acceptable_time_without_backpressure =
            Duration::from_secs(ACCEPTABLE_TIME_WITHOUT_BACKPRESSURE_SECS);

        now > self.time_when_backpressure_detected + acceptable_time_without_backpressure
            && now > self.time_when_changed + delay_change_interval
    }
}

pub(crate) struct OutQueueControl<R>
where
    R: CryptoRng + Rng,
{
    /// Configurable parameters of the `ActionController`
    config: Config,

    /// Key used to encrypt and decrypt content of an ACK packet.
    ack_key: Arc<AckKey>,

    /// Channel used for notifying of a real packet being sent out. Used to start up retransmission timer.
    sent_notifier: SentPacketNotificationSender,

    /// Internal state, determined by `average_message_sending_delay`,
    /// used to keep track of when a next packet should be sent out.
    #[cfg(not(target_arch = "wasm32"))]
    next_delay: Option<Pin<Box<time::Sleep>>>,

    #[cfg(target_arch = "wasm32")]
    next_delay: Option<Pin<Box<wasm_timer::Delay>>>,

    // To make sure we don't overload the mix_tx channel, we limit the rate we are pushing
    // messages.
    sending_rate_controller: SendingDelayController,

    /// Channel used for sending prepared sphinx packets to `MixTrafficController` that sends them
    /// out to the network without any further delays.
    mix_tx: BatchMixMessageSender,

    /// Channel used for receiving real, prepared, messages that must be first sufficiently delayed
    /// before being sent out into the network.
    real_receiver: BatchRealMessageReceiver,

    /// Represents full address of this client.
    our_full_destination: Recipient,

    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Accessor to the common instance of network topology.
    topology_access: TopologyAccessor,

    /// Buffer containing all real messages keyed by connection id
    //received_buffer: HashMap<TransmissionLane, VecDeque<RealMessage>>,
    //received_buffer: HashMap<TransmissionLane, LaneBufferEntry>,
    received_buffer: ReceivedBuffer,
}

#[derive(Default)]
struct ReceivedBuffer {
    buffer: HashMap<TransmissionLane, LaneBufferEntry>,
}

impl ReceivedBuffer {
    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn total_size(&self) -> usize {
        self.buffer.values().map(|v| v.len()).sum()
    }

    fn store(&mut self, lane: &TransmissionLane, real_messages: Vec<RealMessage>) {
        //let prev_msgs = self.buffer.entry(lane).or_default();
        //prev_msgs.append(&mut real_messages.into());

        if let Some(lane_buffer_entry) = self.buffer.get_mut(lane) {
            lane_buffer_entry.append(real_messages);
        } else {
            self.buffer
                .insert(lane.clone(), LaneBufferEntry::new(real_messages));
        }
    }

    fn pick_random_lane(&self) -> Option<&TransmissionLane> {
        // Pick one connection at random to return a stream message from
        let lanes: Vec<&TransmissionLane> = self.buffer.keys().collect();
        log::info!("number of lanes to choose from: {}", lanes.len());
        lanes.choose(&mut rand::thread_rng()).copied()
    }

    fn pick_random_small_lane(&self) -> Option<&TransmissionLane> {
        // Pick one connection at random to return a stream message from
        let lanes: Vec<&TransmissionLane> = self
            .buffer
            .iter()
            .filter(|(_, v)| v.is_small())
            .map(|(k, _)| k)
            .collect();
        log::info!("number of (small) lanes to choose from: {}", lanes.len());
        lanes.choose(&mut rand::thread_rng()).copied()
    }

    fn pick_random_old_lane(&self) -> Option<&TransmissionLane> {
        let lanes: Vec<&TransmissionLane> = self
            .buffer
            .iter()
            .filter(|(_, v)| v.is_old())
            .map(|(k, _)| k)
            .collect();
        log::info!("number of (old) lanes to choose from: {}", lanes.len());
        lanes.choose(&mut rand::thread_rng()).copied()
    }

    fn pop_from_lane(&mut self, lane: &TransmissionLane) -> Option<RealMessage> {
        let real_msgs_queued = self.buffer.get_mut(lane)?;
        let real_next = real_msgs_queued.pop_front()?;
        if real_msgs_queued.is_empty() {
            self.buffer.remove(lane);
        }
        Some(real_next)
    }

    fn pop_next_message_at_random(&mut self) -> Option<RealMessage> {
        //let values = self.received_buffer.values();
        //let packet_backlog = values.count();

        if self.buffer.is_empty() {
            return None;
        }

        log::info!("List all received_buffers");
        for (k, v) in &self.buffer {
            log::info!("{:?}: packets: {}", k, v.len());
        }
        let total = self.total_size();
        log::info!("Total: {}", total);
        log::info!("sec left: {}", total as f64 / 50.0);
        log::info!("min left: {}", total as f64 / 50.0 / 60.0);

        let lane = if let Some(small_lane) = self.pick_random_small_lane() {
            small_lane.clone()
        } else if let Some(old_lane) = self.pick_random_old_lane() {
            old_lane.clone()
        } else {
            self.pick_random_lane()?.clone()
        };

        //let lane = self.pick_random_lane()?.clone();
        log::info!("picking to send from lane: {:?}", lane);

        self.pop_from_lane(&lane)

        //// We just picked a valid lane, and returned early if none existed.
        //let real_msgs_queued = self.buffer.get_mut(&lane).unwrap();
        //// If an entry exists, it has non-zero number of messages.
        //let real_next = real_msgs_queued.pop_front().unwrap();
        //if real_msgs_queued.is_empty() {
        //    self.buffer.remove(&lane);
        //}
        //Some(real_next)
    }
}

struct LaneBufferEntry {
    pub real_messages: VecDeque<RealMessage>,
    pub time_for_first_activity: time::Instant,
    pub time_for_last_activity: time::Instant,
}

impl LaneBufferEntry {
    fn new(real_messages: Vec<RealMessage>) -> Self {
        let now = time::Instant::now();
        LaneBufferEntry {
            real_messages: real_messages.into(),
            time_for_first_activity: now,
            time_for_last_activity: now,
        }
    }

    fn append(&mut self, real_messages: Vec<RealMessage>) {
        self.real_messages.append(&mut real_messages.into());
        self.time_for_last_activity = time::Instant::now();
    }

    fn pop_front(&mut self) -> Option<RealMessage> {
        self.real_messages.pop_front()
    }

    fn is_small(&self) -> bool {
        self.real_messages.len() < 100
    }

    fn is_old(&self) -> bool {
        time::Instant::now() - self.time_for_first_activity > Duration::from_secs(5)
    }

    fn is_stale(&self) -> bool {
        time::Instant::now() - self.time_for_last_activity > Duration::from_secs(30)
    }

    fn len(&self) -> usize {
        self.real_messages.len()
    }

    fn is_empty(&self) -> bool {
        self.real_messages.is_empty()
    }
}

pub(crate) struct RealMessage {
    mix_packet: MixPacket,
    fragment_id: FragmentIdentifier,
}

impl RealMessage {
    pub(crate) fn new(mix_packet: MixPacket, fragment_id: FragmentIdentifier) -> Self {
        RealMessage {
            mix_packet,
            fragment_id,
        }
    }
}

// messages are already prepared, etc. the real point of it is to forward it to mix_traffic
// after sufficient delay
pub(crate) type BatchRealMessageSender =
    mpsc::UnboundedSender<(Vec<RealMessage>, TransmissionLane)>;
type BatchRealMessageReceiver = mpsc::UnboundedReceiver<(Vec<RealMessage>, TransmissionLane)>;

pub(crate) enum StreamMessage {
    Cover,
    Real(Box<RealMessage>),
}

impl<R> OutQueueControl<R>
where
    R: CryptoRng + Rng + Unpin,
{
    // at this point I'm not entirely sure how to deal with this warning without
    // some considerable refactoring
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        config: Config,
        ack_key: Arc<AckKey>,
        sent_notifier: SentPacketNotificationSender,
        mix_tx: BatchMixMessageSender,
        real_receiver: BatchRealMessageReceiver,
        rng: R,
        our_full_destination: Recipient,
        topology_access: TopologyAccessor,
    ) -> Self {
        OutQueueControl {
            config,
            ack_key,
            sent_notifier,
            next_delay: None,
            sending_rate_controller: SendingDelayController::new(
                MIN_DELAY_MULTIPLIER,
                MAX_DELAY_MULTIPLIER,
            ),
            mix_tx,
            real_receiver,
            our_full_destination,
            rng,
            topology_access,
            received_buffer: Default::default(),
        }
    }

    fn sent_notify(&self, frag_id: FragmentIdentifier) {
        // well technically the message was not sent just yet, but now it's up to internal
        // queues and client load rather than the required delay. So realistically we can treat
        // whatever is about to happen as negligible additional delay.
        trace!("{} is about to get sent to the mixnet", frag_id);
        self.sent_notifier.unbounded_send(frag_id).unwrap();
    }

    async fn on_message(&mut self, next_message: StreamMessage) {
        trace!("created new message");

        let (next_message, fragment_id) = match next_message {
            StreamMessage::Cover => {
                // TODO for way down the line: in very rare cases (during topology update) we might have
                // to wait a really tiny bit before actually obtaining the permit hence messing with our
                // poisson delay, but is it really a problem?
                let topology_permit = self.topology_access.get_read_permit().await;
                // the ack is sent back to ourselves (and then ignored)
                let topology_ref_option = topology_permit.try_get_valid_topology_ref(
                    &self.our_full_destination,
                    Some(&self.our_full_destination),
                );
                if topology_ref_option.is_none() {
                    warn!(
                        "No valid topology detected - won't send any loop cover message this time"
                    );
                    return;
                }
                let topology_ref = topology_ref_option.unwrap();

                (
                    generate_loop_cover_packet(
                        &mut self.rng,
                        topology_ref,
                        &self.ack_key,
                        &self.our_full_destination,
                        self.config.average_ack_delay,
                        self.config.average_packet_delay,
                        self.config.cover_packet_size,
                    )
                    .expect(
                        "Somehow failed to generate a loop cover message with a valid topology",
                    ),
                    None,
                )
            }
            StreamMessage::Real(real_message) => {
                (real_message.mix_packet, Some(real_message.fragment_id))
            }
        };

        if let Err(err) = self.mix_tx.send(vec![next_message]).await {
            log::error!("Failed to send - channel closed: {}", err);
        }

        // notify ack controller about sending our message only after we actually managed to push it
        // through the channel
        if let Some(fragment_id) = fragment_id {
            self.sent_notify(fragment_id);
        }

        // JS: Not entirely sure why or how it fixes stuff, but without the yield call,
        // the UnboundedReceiver [of mix_rx] will not get a chance to read anything
        // JS2: Basically it was the case that with high enough rate, the stream had already a next value
        // ready and hence was immediately re-scheduled causing other tasks to be starved;
        // yield makes it go back the scheduling queue regardless of its value availability

        // TODO: temporary and BAD workaround for wasm (we should find a way to yield here in wasm)
        #[cfg(not(target_arch = "wasm32"))]
        tokio::task::yield_now().await;
    }

    fn current_average_message_sending_delay(&self) -> Duration {
        self.config.average_message_sending_delay
            * self.sending_rate_controller.current_multiplier()
    }

    fn adjust_current_average_message_sending_delay(&mut self) {
        let used_slots = self.mix_tx.max_capacity() - self.mix_tx.capacity();
        log::trace!(
            "used_slots: {used_slots}, current_multiplier: {}",
            self.sending_rate_controller.current_multiplier()
        );

        // Even just a single used slot is enough to signal backpressure
        if used_slots > 0 {
            log::trace!("Backpressure detected");
            self.sending_rate_controller.record_backpressure_detected();
        }

        // If the buffer is running out, slow down the sending rate
        if self.mix_tx.capacity() == 0
            && self.sending_rate_controller.not_increased_delay_recently()
        {
            self.sending_rate_controller.increase_delay_multiplier();
        }

        // Very carefully step up the sending rate in case it seems like we can solidly handle the
        // current rate.
        if self.sending_rate_controller.is_sending_reliable() {
            self.sending_rate_controller.decrease_delay_multiplier();
        }
    }

    //fn store_real_messages(&mut self, lane: TransmissionLane, real_messages: Vec<RealMessage>) {
    //    let prev_msgs = self.received_buffer.entry(lane).or_default();
    //    prev_msgs.append(&mut real_messages.into());
    //}

    //fn pick_random_lane(&self) -> Option<&TransmissionLane> {
    //    // Pick one connection at random to return a stream message from
    //    let lanes: Vec<&TransmissionLane> = self.received_buffer.keys().collect();
    //    log::info!("number of lanes to choose from: {}", lanes.len());
    //    lanes.choose(&mut rand::thread_rng()).copied()
    //}

    //fn pick_random_small_lane(&self) -> Option<&TransmissionLane> {
    //    // Pick one connection at random to return a stream message from
    //    let lanes: Vec<&TransmissionLane> = self
    //        .received_buffer
    //        .iter()
    //        .filter(|(_, v)| v.len() < 100)
    //        .map(|(k, _)| k)
    //        .collect();
    //    log::info!("number of (small) lanes to choose from: {}", lanes.len());
    //    lanes.choose(&mut rand::thread_rng()).copied()
    //}

    // Get the next real message, and remove the transmission lane entry if it was the last one.
    //fn pop_next_message_at_random(&mut self) -> Option<RealMessage> {
    //    //let values = self.received_buffer.values();
    //    //let packet_backlog = values.count();
    //    log::info!("List all received_buffers");
    //    for (k, v) in &self.received_buffer {
    //        log::info!("{:?}: packets: {}", k, v.len());
    //    }
    //    let total: usize = self.received_buffer.values().map(|v| v.len()).sum();
    //    log::info!("Total: {}", total);
    //    log::info!("sec left: {}", total as f64 / 50.0);
    //    log::info!("min left: {}", total as f64 / 50.0 / 60.0);

    //    let lane = if let Some(small_lane) = self.pick_random_small_lane() {
    //        small_lane.clone()
    //    } else {
    //        self.pick_random_lane()?.clone()
    //    };

    //    //let lane = self.pick_random_lane()?.clone();
    //    log::info!("picking to send from lane: {:?}", lane);

    //    // We just picked a valid lane, and returned early if none existed.
    //    let real_msgs_queued = self.received_buffer.get_mut(&lane).unwrap();
    //    // If an entry exists, it has non-zero number of messages.
    //    let real_next = real_msgs_queued.pop_front().unwrap();
    //    if real_msgs_queued.is_empty() {
    //        self.received_buffer.remove(&lane);
    //    }
    //    Some(real_next)
    //}

    fn poll_poisson(&mut self, cx: &mut Context<'_>) -> Poll<Option<StreamMessage>> {
        // The average delay could change depending on if backpressure in the downstream channel
        // (mix_tx) was detected.
        self.adjust_current_average_message_sending_delay();
        let avg_delay = self.current_average_message_sending_delay();

        if let Some(ref mut next_delay) = &mut self.next_delay {
            // it is not yet time to return a message
            if next_delay.as_mut().poll(cx).is_pending() {
                return Poll::Pending;
            };

            // we know it's time to send a message, so let's prepare delay for the next one
            // Get the `now` by looking at the current `delay` deadline
            let next_poisson_delay = sample_poisson_duration(&mut self.rng, avg_delay);

            // The next interval value is `next_poisson_delay` after the one that just
            // yielded.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let now = next_delay.deadline();
                let next = now + next_poisson_delay;
                next_delay.as_mut().reset(next);
            }

            #[cfg(target_arch = "wasm32")]
            {
                next_delay.as_mut().reset(next_poisson_delay);
            }

            // WIP(JON): prioritize connections according to:
            // 1. small amount of data
            // 2. large chunks of data which have been waiting for a longer time
            // 3. large chunks of data in new connections

            // max number of connections
            //if self.received_buffer.len() > 10 {
            //    let real_next = self.pop_next_message_at_random().expect("we just checked");

            //    return Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))));
            //}

            match Pin::new(&mut self.real_receiver).poll_next(cx) {
                // in the case our real message channel stream was closed, we should also indicate we are closed
                // (and whoever is using the stream should panic)
                Poll::Ready(None) => Poll::Ready(None),

                Poll::Ready(Some((real_messages, conn_id))) => {
                    log::info!("handing real_messages: size: {}", real_messages.len());

                    self.received_buffer.store(&conn_id, real_messages);
                    let real_next = self
                        .received_buffer
                        .pop_next_message_at_random()
                        .expect("we just added one");

                    Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))))
                }

                Poll::Pending => {
                    if let Some(real_next) = self.received_buffer.pop_next_message_at_random() {
                        // if there are more messages immediately available, notify the runtime
                        // because we should be polled again
                        if !self.received_buffer.is_empty() {
                            cx.waker().wake_by_ref()
                        }

                        Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))))
                    } else {
                        // otherwise construct a dummy one
                        Poll::Ready(Some(StreamMessage::Cover))
                    }
                }
            }
        } else {
            // we never set an initial delay - let's do it now
            cx.waker().wake_by_ref();

            let sampled =
                sample_poisson_duration(&mut self.rng, self.config.average_message_sending_delay);

            #[cfg(not(target_arch = "wasm32"))]
            let next_delay = Box::pin(time::sleep(sampled));

            #[cfg(target_arch = "wasm32")]
            let next_delay = Box::pin(wasm_timer::Delay::new(sampled));

            self.next_delay = Some(next_delay);

            Poll::Pending
        }
    }

    fn poll_immediate(&mut self, cx: &mut Context<'_>) -> Poll<Option<StreamMessage>> {
        // WIP(JON): bound the number of connections we allow to store at the same time

        // WIP(JON): prioritize connections according to:
        // 1. small amount of data
        // 2. large chunks of data which have been waiting for a longer time
        // 3. large chunks of data in new connections

        match Pin::new(&mut self.real_receiver).poll_next(cx) {
            // in the case our real message channel stream was closed, we should also indicate we are closed
            // (and whoever is using the stream should panic)
            Poll::Ready(None) => Poll::Ready(None),

            Poll::Ready(Some((real_messages, conn_id))) => {
                log::info!("handing real_messages: size: {}", real_messages.len());

                // First store what we got for the given connection id
                self.received_buffer.store(&conn_id, real_messages);
                let real_next = self
                    .received_buffer
                    .pop_next_message_at_random()
                    .expect("we just added one");

                // if there are more messages immediately available, notify the runtime
                // because we should be polled again
                //if !self.received_buffer.is_empty() {
                //    cx.waker().wake_by_ref()
                //}

                Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))))
            }

            Poll::Pending => {
                if let Some(real_next) = self.received_buffer.pop_next_message_at_random() {
                    // if there are more messages immediately available, notify the runtime
                    // because we should be polled again
                    //if !self.received_buffer.is_empty() {
                    //    cx.waker().wake_by_ref()
                    //}

                    Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))))
                } else {
                    Poll::Pending
                }
            }
        }
    }

    fn poll_next_message(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<StreamMessage>> {
        if self.config.disable_poisson_packet_distribution {
            self.poll_immediate(cx)
        } else {
            self.poll_poisson(cx)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: task::ShutdownListener) {
        debug!("Started OutQueueControl with graceful shutdown support");

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("OutQueueControl: Received shutdown");
                }
                next_message = self.next() => match next_message {
                    Some(next_message) => {
                        self.on_message(next_message).await;
                    },
                    None => {
                        log::trace!("OutQueueControl: Stopping since channel closed");
                        break;
                    }
                }
            }
        }
        assert!(shutdown.is_shutdown_poll());
        log::debug!("OutQueueControl: Exiting");
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) async fn run(&mut self) {
        debug!("Started OutQueueControl without graceful shutdown support");

        while let Some(next_message) = self.next().await {
            self.on_message(next_message).await;
        }
    }
}

impl<R> Stream for OutQueueControl<R>
where
    R: CryptoRng + Rng + Unpin,
{
    type Item = StreamMessage;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll_next_message(cx)
    }
}
