// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::sending_delay_controller::SendingDelayController;
use crate::client::mix_traffic::BatchMixMessageSender;
use crate::client::packet_statistics_control::{PacketStatisticsEvent, PacketStatisticsReporter};
use crate::client::real_messages_control::acknowledgement_control::SentPacketNotificationSender;
use crate::client::topology_control::TopologyAccessor;
use crate::client::transmission_buffer::TransmissionBuffer;
use crate::config;
use futures::channel::mpsc;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::*;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::chunking::fragment::FragmentIdentifier;
use nym_sphinx::cover::{generate_drop_cover_packet, generate_loop_cover_packet};
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketSize;
use nym_sphinx::preparer::PreparedFragment;
use nym_sphinx::utils::sample_poisson_duration;
use nym_task::connections::{
    ConnectionCommand, ConnectionCommandReceiver, ConnectionId, LaneQueueLengths, TransmissionLane,
};
use rand::{CryptoRng, Rng};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{sleep, Sleep};

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::{sleep, Sleep};

mod sending_delay_controller;

/// Configurable parameters of the `OutQueueControl`
pub(crate) struct Config {
    /// Key used to encrypt and decrypt content of an ACK packet.
    ack_key: Arc<AckKey>,

    /// Represents full address of this client.
    our_full_destination: Recipient,

    /// Average delay an acknowledgement packet is going to get delay at a single mixnode.
    average_ack_delay: Duration,

    /// Defines all configuration options related to this traffic stream.
    traffic: config::Traffic,

    /// Specifies the ratio of `primary_packet_size` to `secondary_packet_size` used in cover traffic.
    /// Only applicable if `secondary_packet_size` is enabled.
    cover_traffic_primary_size_ratio: f64,
}

impl Config {
    pub(crate) fn new(
        ack_key: Arc<AckKey>,
        our_full_destination: Recipient,
        average_ack_delay: Duration,
        traffic: config::Traffic,
        cover_traffic_primary_size_ratio: f64,
    ) -> Self {
        Config {
            ack_key,
            our_full_destination,
            average_ack_delay,
            traffic,
            cover_traffic_primary_size_ratio,
        }
    }
}

pub(crate) struct OutQueueControl<R>
where
    R: CryptoRng + Rng,
{
    /// Configurable parameters of the `ActionController`
    config: Config,

    /// Channel used for notifying of a real packet being sent out. Used to start up retransmission timer.
    sent_notifier: SentPacketNotificationSender,

    /// Internal state, determined by `average_message_sending_delay`,
    /// used to keep track of when a next packet should be sent out.
    next_delay: Option<Pin<Box<Sleep>>>,

    // To make sure we don't overload the mix_tx channel, we limit the rate we are pushing
    // messages.
    sending_delay_controller: SendingDelayController,

    /// Channel used for sending prepared packets to `MixTrafficController` that sends them
    /// out to the network without any further delays.
    mix_tx: BatchMixMessageSender,

    /// Channel used for receiving real, prepared, messages that must be first sufficiently delayed
    /// before being sent out into the network.
    real_receiver: BatchRealMessageReceiver,

    /// Instance of a cryptographically secure random number generator.
    rng: R,

    /// Accessor to the common instance of network topology.
    topology_access: TopologyAccessor,

    /// Buffer containing all incoming real messages keyed by transmission lane, that we will send
    /// out to the mixnet.
    transmission_buffer: TransmissionBuffer<RealMessage>,

    /// Incoming channel for being notified of closed connections, so that we can close lanes
    /// corresponding to connections. To avoid sending traffic unnecessary
    client_connection_rx: ConnectionCommandReceiver,

    /// Report queue lengths so that upstream can backoff sending data, and keep connections open.
    lane_queue_lengths: LaneQueueLengths,

    /// Channel used for sending statistics events to `PacketStatisticsControl`.
    stats_tx: PacketStatisticsReporter,
    //counter for drop cover traffic
    counter_receiver: mpsc::Receiver<u8>,
}

#[derive(Debug)]
pub(crate) struct RealMessage {
    mix_packet: MixPacket,
    fragment_id: Option<FragmentIdentifier>,
    // TODO: add info about it being constructed with reply-surb
}

impl From<PreparedFragment> for RealMessage {
    fn from(fragment: PreparedFragment) -> Self {
        RealMessage {
            mix_packet: fragment.mix_packet,
            fragment_id: Some(fragment.fragment_identifier),
        }
    }
}

impl RealMessage {
    pub(crate) fn packet_size(&self) -> usize {
        self.mix_packet.packet().len()
    }

    pub(crate) fn new(mix_packet: MixPacket, fragment_id: Option<FragmentIdentifier>) -> Self {
        RealMessage {
            mix_packet,
            fragment_id,
        }
    }
}

// messages are already prepared, etc. the real point of it is to forward it to mix_traffic
// after sufficient delay
pub(crate) type BatchRealMessageSender =
    tokio::sync::mpsc::Sender<(Vec<RealMessage>, TransmissionLane)>;
type BatchRealMessageReceiver = tokio::sync::mpsc::Receiver<(Vec<RealMessage>, TransmissionLane)>;

pub(crate) enum StreamMessage {
    Cover(bool),
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
        rng: R,
        sent_notifier: SentPacketNotificationSender,
        mix_tx: BatchMixMessageSender,
        real_receiver: BatchRealMessageReceiver,
        topology_access: TopologyAccessor,
        lane_queue_lengths: LaneQueueLengths,
        client_connection_rx: ConnectionCommandReceiver,
        stats_tx: PacketStatisticsReporter,
        counter_receiver: mpsc::Receiver<u8>,
    ) -> Self {
        OutQueueControl {
            config,
            sent_notifier,
            next_delay: None,
            sending_delay_controller: Default::default(),
            mix_tx,
            real_receiver,
            rng,
            topology_access,
            transmission_buffer: TransmissionBuffer::new(),
            client_connection_rx,
            lane_queue_lengths,
            stats_tx,
            counter_receiver,
        }
    }

    fn sent_notify(&self, frag_id: FragmentIdentifier) {
        // well technically the message was not sent just yet, but now it's up to internal
        // queues and client load rather than the required delay. So realistically we can treat
        // whatever is about to happen as negligible additional delay.
        trace!("{} is about to get sent to the mixnet", frag_id);
        self.sent_notifier.unbounded_send(frag_id).unwrap();
    }

    fn loop_cover_message_size(&mut self) -> PacketSize {
        let Some(secondary_packet_size) = self.config.traffic.secondary_packet_size else {
            return self.config.traffic.primary_packet_size;
        };

        let use_primary = self
            .rng
            .gen_bool(self.config.cover_traffic_primary_size_ratio);

        if use_primary {
            self.config.traffic.primary_packet_size
        } else {
            secondary_packet_size
        }
    }

    async fn on_message(&mut self, next_message: StreamMessage) {
        trace!("created new message");

        let (next_message, fragment_id, packet_size) = match next_message {
            StreamMessage::Cover(drop) => {
                let cover_traffic_packet_size = self.loop_cover_message_size();
                trace!("the next loop cover message will be put in a {cover_traffic_packet_size} packet");

                // TODO for way down the line: in very rare cases (during topology update) we might have
                // to wait a really tiny bit before actually obtaining the permit hence messing with our
                // poisson delay, but is it really a problem?
                let topology_permit = self.topology_access.get_read_permit().await;
                // the ack is sent back to ourselves (and then ignored)
                let topology_ref = match topology_permit.try_get_valid_topology_ref(
                    &self.config.our_full_destination,
                    Some(&self.config.our_full_destination),
                ) {
                    Ok(topology) => topology,
                    Err(err) => {
                        warn!("We're not going to send any loop cover message this time, as the current topology seem to be invalid - {err}");
                        return;
                    }
                };
                if drop {
                    debug!("Sending a drop cover message");
                    (
                        generate_drop_cover_packet(
                            &mut self.rng,
                            topology_ref,
                            &self.config.ack_key,
                            &self.config.our_full_destination,
                            self.config.average_ack_delay,
                            self.config.traffic.average_packet_delay,
                            cover_traffic_packet_size,
                            self.config.traffic.packet_type,
                        )
                        .expect(
                            "Somehow failed to generate a drop cover message with a valid topology",
                        ),
                        None,
                        cover_traffic_packet_size.size(),
                    )
                } else {
                    (
                        generate_loop_cover_packet(
                            &mut self.rng,
                            topology_ref,
                            &self.config.ack_key,
                            &self.config.our_full_destination,
                            self.config.average_ack_delay,
                            self.config.traffic.average_packet_delay,
                            cover_traffic_packet_size,
                            self.config.traffic.packet_type,
                        )
                        .expect(
                            "Somehow failed to generate a loop cover message with a valid topology",
                        ),
                        None,
                        cover_traffic_packet_size.size(),
                    )
                }
            }
            StreamMessage::Real(real_message) => {
                let packet_size = real_message.packet_size();
                (
                    real_message.mix_packet,
                    real_message.fragment_id,
                    packet_size,
                )
            }
        };

        if let Err(err) = self.mix_tx.send(vec![next_message]).await {
            log::error!("Failed to send: {err}");
        } else {
            let event = if fragment_id.is_some() {
                PacketStatisticsEvent::RealPacketSent(packet_size)
            } else {
                PacketStatisticsEvent::CoverPacketSent(packet_size)
            };
            self.stats_tx.report(event);
        }

        // notify ack controller about sending our message only after we actually managed to push it
        // through the channel
        if let Some(fragment_id) = fragment_id {
            self.sent_notify(fragment_id);
        }

        // In addition to closing connections on receiving messages throught client_connection_rx,
        // also close connections when sufficiently stale.
        self.transmission_buffer.prune_stale_connections();

        // JS: Not entirely sure why or how it fixes stuff, but without the yield call,
        // the UnboundedReceiver [of mix_rx] will not get a chance to read anything
        // JS2: Basically it was the case that with high enough rate, the stream had already a next value
        // ready and hence was immediately re-scheduled causing other tasks to be starved;
        // yield makes it go back the scheduling queue regardless of its value availability

        // TODO: temporary and BAD workaround for wasm (we should find a way to yield here in wasm)
        #[cfg(not(target_arch = "wasm32"))]
        tokio::task::yield_now().await;
    }

    fn on_close_connection(&mut self, connection_id: ConnectionId) {
        log::debug!("Removing lane for connection: {connection_id}");
        self.transmission_buffer
            .remove(&TransmissionLane::ConnectionId(connection_id));
    }

    fn current_average_message_sending_delay(&self) -> Duration {
        self.config.traffic.message_sending_average_delay
            * self.sending_delay_controller.current_multiplier()
    }

    fn adjust_current_average_message_sending_delay(&mut self) {
        let used_slots = self.mix_tx.max_capacity() - self.mix_tx.capacity();
        log::trace!(
            "used_slots: {used_slots}, current_multiplier: {}",
            self.sending_delay_controller.current_multiplier()
        );

        if self
            .sending_delay_controller
            .is_backpressure_currently_detected(used_slots)
        {
            log::trace!("Backpressure detected");
            self.sending_delay_controller.record_backpressure_detected();
        }

        // If the buffer is running out, slow down the sending rate by increasing the delay
        // multiplier.
        if self.mix_tx.capacity() == 0
            && self.sending_delay_controller.not_increased_delay_recently()
        {
            self.sending_delay_controller.increase_delay_multiplier();
        }

        // If it looks like we are sending reliably, increase the sending rate by decreasing the
        // sending delay multiplier.
        if !self
            .sending_delay_controller
            .was_backpressure_detected_recently()
            && self.sending_delay_controller.not_decreased_delay_recently()
        {
            self.sending_delay_controller.decrease_delay_multiplier();
        }

        // Keep track of multiplier changes, and log if necessary.
        self.sending_delay_controller.record_delay_multiplier();
    }

    fn pop_next_message(&mut self) -> Option<RealMessage> {
        // Pop the next message from the transmission buffer
        let (lane, real_next) = self
            .transmission_buffer
            .pop_next_message_at_random(&mut self.rng)?;

        // Update the published queue length
        let lane_length = self.transmission_buffer.lane_length(&lane);
        self.lane_queue_lengths.set(&lane, lane_length);

        // This is the last step in the pipeline where we know the type of the message, so
        // lets count the number of retransmissions and reply surb messages sent here.
        let stat_event = match lane {
            TransmissionLane::General => None,
            TransmissionLane::ConnectionId(_) => None,
            TransmissionLane::ReplySurbRequest => {
                Some(PacketStatisticsEvent::ReplySurbRequestQueued)
            }
            TransmissionLane::AdditionalReplySurbs => {
                Some(PacketStatisticsEvent::AdditionalReplySurbRequestQueued)
            }
            TransmissionLane::Retransmission => Some(PacketStatisticsEvent::RetransmissionQueued),
        };
        if let Some(stat_event) = stat_event {
            self.stats_tx.report(stat_event);
        }
        // To avoid comparing apples to oranges when presenting the fraction of packets that are
        // retransmissions, we also need to keep track to the total number of real messages queued,
        // even though we also track the actual number of messages sent later in the pipeline.
        self.stats_tx
            .report(PacketStatisticsEvent::RealPacketQueued);

        Some(real_next)
    }

    fn poll_poisson(&mut self, cx: &mut Context<'_>) -> Poll<Option<StreamMessage>> {
        //For instantaneous drop cover traffic
        // if let Ok(_) = self.counter_receiver.try_next() {
        //     return Poll::Ready(Some(StreamMessage::Cover(true)));
        // };
        // The average delay could change depending on if backpressure in the downstream channel
        // (mix_tx) was detected.
        self.adjust_current_average_message_sending_delay();
        let avg_delay = self.current_average_message_sending_delay();

        // Start by checking if we have any incoming messages about closed connections
        // NOTE: this feels a bit iffy, the `OutQueueControl` is getting ripe for a rewrite to
        // something simpler.
        if let Poll::Ready(Some(id)) = Pin::new(&mut self.client_connection_rx).poll_next(cx) {
            match id {
                ConnectionCommand::Close(id) => self.on_close_connection(id),
            }
        }

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
            let now = next_delay.deadline();
            let next = now + next_poisson_delay;
            next_delay.as_mut().reset(next);

            // On every iteration we get new messages from upstream. Given that these come bunched
            // in `Vec`, this ensures that on average we will fetch messages faster than we can
            // send, which is a condition for being able to multiplex packets from multiple
            // data streams.
            let need_drop = self.counter_receiver.try_next();
            match Pin::new(&mut self.real_receiver).poll_recv(cx) {
                // in the case our real message channel stream was closed, we should also indicate we are closed
                // (and whoever is using the stream should panic)
                Poll::Ready(None) => Poll::Ready(None),

                Poll::Ready(Some((real_messages, conn_id))) => {
                    log::trace!("handling real_messages: size: {}", real_messages.len());

                    self.transmission_buffer.store(&conn_id, real_messages);
                    let real_next = self.pop_next_message().expect("Just stored one");

                    Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))))
                }

                Poll::Pending => {
                    if let Some(real_next) = self.pop_next_message() {
                        Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))))
                    } else {
                        // otherwise construct a dummy one
                        match need_drop {
                            Ok(_) => Poll::Ready(Some(StreamMessage::Cover(true))),
                            _ => Poll::Ready(Some(StreamMessage::Cover(false))),
                        }
                    }
                }
            }
        } else {
            // we never set an initial delay - let's do it now
            cx.waker().wake_by_ref();

            let sampled = sample_poisson_duration(
                &mut self.rng,
                self.config.traffic.message_sending_average_delay,
            );

            let next_delay = Box::pin(sleep(sampled));
            self.next_delay = Some(next_delay);

            Poll::Pending
        }
    }

    fn poll_immediate(&mut self, cx: &mut Context<'_>) -> Poll<Option<StreamMessage>> {
        // Start by checking if we have any incoming messages about closed connections
        if let Poll::Ready(Some(id)) = Pin::new(&mut self.client_connection_rx).poll_next(cx) {
            match id {
                ConnectionCommand::Close(id) => self.on_close_connection(id),
            }
        }

        match Pin::new(&mut self.real_receiver).poll_recv(cx) {
            // in the case our real message channel stream was closed, we should also indicate we are closed
            // (and whoever is using the stream should panic)
            Poll::Ready(None) => Poll::Ready(None),

            Poll::Ready(Some((real_messages, conn_id))) => {
                log::trace!("handling real_messages: size: {}", real_messages.len());

                // This is the last step in the pipeline where we know the type of the message, so
                // lets count the number of retransmissions here.
                if conn_id == TransmissionLane::Retransmission {
                    self.stats_tx
                        .report(PacketStatisticsEvent::RetransmissionQueued);
                }

                // First store what we got for the given connection id
                self.transmission_buffer.store(&conn_id, real_messages);
                let real_next = self.pop_next_message().expect("we just added one");

                Poll::Ready(Some(StreamMessage::Real(Box::new(real_next))))
            }

            Poll::Pending => {
                if let Some(real_next) = self.pop_next_message() {
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
        if self.config.traffic.disable_main_poisson_packet_distribution {
            self.poll_immediate(cx)
        } else {
            self.poll_poisson(cx)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn log_status(&self, shutdown: &mut nym_task::TaskClient) {
        use crate::error::ClientCoreStatusMessage;

        let packets = self.transmission_buffer.total_size();
        let backlog = self.transmission_buffer.total_size_in_bytes() as f64 / 1024.0;
        let lanes = self.transmission_buffer.num_lanes();
        let mult = self.sending_delay_controller.current_multiplier();
        let delay = self.current_average_message_sending_delay().as_millis();
        let status_str = if self.config.traffic.disable_main_poisson_packet_distribution {
            format!("Packet backlog: {backlog:.2} kiB ({packets}), {lanes} lanes, no delay")
        } else {
            format!(
                "Packet backlog: {backlog:.2} kiB ({packets}), {lanes} lanes, avg delay: {delay}ms ({mult})"
            )
        };
        if packets > 1000 {
            log::warn!("{status_str}");
        } else if packets > 0 {
            log::info!("{status_str}");
        } else {
            log::debug!("{status_str}");
        }

        // Send status message to whoever is listening (possibly UI)
        if mult == self.sending_delay_controller.max_multiplier() {
            shutdown.send_status_msg(Box::new(ClientCoreStatusMessage::GatewayIsVerySlow));
        } else if mult > self.sending_delay_controller.min_multiplier() {
            shutdown.send_status_msg(Box::new(ClientCoreStatusMessage::GatewayIsSlow));
        }
    }

    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        debug!("Started OutQueueControl with graceful shutdown support");

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut status_timer = tokio::time::interval(Duration::from_secs(5));

            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.recv_with_delay() => {
                        log::trace!("OutQueueControl: Received shutdown");
                        break;
                    }
                    _ = status_timer.tick() => {
                        self.log_status(&mut shutdown);
                    }
                    next_message = self.next() => if let Some(next_message) = next_message {
                        self.on_message(next_message).await;
                    } else {
                        log::trace!("OutQueueControl: Stopping since channel closed");
                        break;
                    }
                }
            }
            shutdown.recv_timeout().await;
        }

        #[cfg(target_arch = "wasm32")]
        {
            while !shutdown.is_shutdown() {
                tokio::select! {
                    biased;
                    _ = shutdown.recv() => {
                        log::trace!("OutQueueControl: Received shutdown");
                    }
                    next_message = self.next() => if let Some(next_message) = next_message {
                        self.on_message(next_message).await;
                    } else {
                        log::trace!("OutQueueControl: Stopping since channel closed");
                        break;
                    }
                }
            }
        }
        log::debug!("OutQueueControl: Exiting");
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
