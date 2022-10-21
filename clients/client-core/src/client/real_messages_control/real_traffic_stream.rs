// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
use rand::{CryptoRng, Rng};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

#[cfg(target_arch = "wasm32")]
use wasm_timer;

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
    time_when_changed: Instant,

    /// If we have a long enough time without any backpressure detected we try reducing the sending
    /// delay multiplier
    time_when_backpressure_detected: Instant,
}

impl SendingDelayController {
    fn new(lower_bound: u32, upper_bound: u32) -> Self {
        assert!(lower_bound <= upper_bound);
        SendingDelayController {
            current_multiplier: 1,
            upper_bound,
            lower_bound,
            time_when_changed: Instant::now(),
            time_when_backpressure_detected: Instant::now(),
        }
    }

    fn increase_delay_multiplier(&mut self) {
        self.current_multiplier =
            (self.current_multiplier + 1).clamp(self.lower_bound, self.upper_bound);
        log::debug!(
            "Increasing sending delay multiplier to: {}",
            self.current_multiplier
        );
    }

    fn decrease_delay_multiplier(&mut self) {
        self.current_multiplier =
            (self.current_multiplier - 1).clamp(self.lower_bound, self.upper_bound);
        log::debug!(
            "Decreasing sending delay multiplier to: {}",
            self.current_multiplier
        );
    }

    fn current_multiplier(&self) -> u32 {
        self.current_multiplier
    }

    // A very basic heuristic to determine the sending rate, using some parameters that potentially
    // might need tweaking.
    fn adjust_multiplier(&mut self, used_slots: usize) {
        log::trace!("used_slots: {used_slots}");
        let now = Instant::now();

        // Whenever we detect that the channel is non-empty we flag for backpressure. This mostly
        // affects how quickly we decrease the delay (increase speeds).
        if used_slots > 0 {
            self.time_when_backpressure_detected = now;
        }

        // As soon as we're above a basic threshold, increase multiplier. But not too often as we
        // need to give time to give the channel a chance to clear.
        if used_slots > 4 && now - self.time_when_changed > Duration::from_millis(500) {
            self.increase_delay_multiplier();
            self.time_when_changed = now;
        }

        // If running smoothly without any backpressure detected, lower the delay multiplier, but
        // not too fast!
        if now - self.time_when_backpressure_detected > Duration::from_secs(2)
            && now - self.time_when_changed > Duration::from_secs(2)
        {
            self.decrease_delay_multiplier();
            self.time_when_changed = now;
        }
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

    /// Buffer containing all real messages received. It is first exhausted before more are pulled.
    received_buffer: VecDeque<RealMessage>,
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
pub(crate) type BatchRealMessageSender = mpsc::UnboundedSender<Vec<RealMessage>>;
type BatchRealMessageReceiver = mpsc::UnboundedReceiver<Vec<RealMessage>>;

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
            sending_rate_controller: SendingDelayController::new(1, 5),
            mix_tx,
            real_receiver,
            our_full_destination,
            rng,
            topology_access,
            received_buffer: VecDeque::with_capacity(0), // we won't be putting any data into this guy directly
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

        let next_message = match next_message {
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

                generate_loop_cover_packet(
                    &mut self.rng,
                    topology_ref,
                    &self.ack_key,
                    &self.our_full_destination,
                    self.config.average_ack_delay,
                    self.config.average_packet_delay,
                    self.config.cover_packet_size,
                )
                .expect("Somehow failed to generate a loop cover message with a valid topology")
            }
            StreamMessage::Real(real_message) => {
                self.sent_notify(real_message.fragment_id);
                real_message.mix_packet
            }
        };

        // if this one fails, there's no retrying because it means that either:
        // - we run out of memory
        // - the receiver channel is closed
        // in either case there's no recovery and we can only panic
        if let Err(err) = self.mix_tx.try_send(vec![next_message]) {
            match err {
                TrySendError::Full(p) => {
                    // Just ignore and drop, it will be resent once the ack timer expires
                    log::warn!("Failed to send {} packets  - channel full)", p.len());
                }
                TrySendError::Closed(p) => {
                    log::warn!("Failed to send {} packets  - channel closed)", p.len());
                }
            }
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
        self.sending_rate_controller.adjust_multiplier(used_slots);
    }

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

            // check if we have anything immediately available
            if let Some(real_available) = self.received_buffer.pop_front() {
                return Poll::Ready(Some(StreamMessage::Real(Box::new(real_available))));
            }

            // decide what kind of message to send
            match Pin::new(&mut self.real_receiver).poll_next(cx) {
                // in the case our real message channel stream was closed, we should also indicate we are closed
                // (and whoever is using the stream should panic)
                Poll::Ready(None) => Poll::Ready(None),

                // if there are more messages available, return first one and store the rest
                Poll::Ready(Some(real_messages)) => {
                    self.received_buffer = real_messages.into();
                    // we MUST HAVE received at least ONE message
                    Poll::Ready(Some(StreamMessage::Real(Box::new(
                        self.received_buffer.pop_front().unwrap(),
                    ))))
                }

                // otherwise construct a dummy one
                Poll::Pending => Poll::Ready(Some(StreamMessage::Cover)),
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
        // check if we have anything immediately available
        if let Some(real_available) = self.received_buffer.pop_front() {
            // if there are more messages immediately available, notify the runtime
            // because we should be polled again
            if !self.received_buffer.is_empty() {
                cx.waker().wake_by_ref()
            }
            return Poll::Ready(Some(StreamMessage::Real(Box::new(real_available))));
        }

        match Pin::new(&mut self.real_receiver).poll_next(cx) {
            // in the case our real message channel stream was closed, we should also indicate we are closed
            // (and whoever is using the stream should panic)
            Poll::Ready(None) => Poll::Ready(None),

            // if there are more messages available, return first one and store the rest
            Poll::Ready(Some(real_messages)) => {
                self.received_buffer = real_messages.into();
                // we MUST HAVE received at least ONE message
                Poll::Ready(Some(StreamMessage::Real(Box::new(
                    self.received_buffer.pop_front().unwrap(),
                ))))
            }

            // if there's nothing, then there's nothing
            Poll::Pending => Poll::Pending,
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
