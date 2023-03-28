// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::sending_delay_controller::SendingDelayController;
use crate::client::mix_traffic::BatchMixMessageSender;
use crate::client::real_messages_control::acknowledgement_control::SentPacketNotificationSender;
use crate::client::topology_control::TopologyAccessor;
use crate::client::transmission_buffer::TransmissionBuffer;
use futures::task::{Context, Poll};
use futures::{Future, Stream, StreamExt};
use log::*;
use nym_sphinx::acknowledgements::AckKey;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::chunking::fragment::FragmentIdentifier;
use nym_sphinx::cover::generate_loop_cover_packet;
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
use std::time::Instant;
use std::thread::sleep;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

#[cfg(target_arch = "wasm32")]
use wasm_timer;

mod sending_delay_controller;

/// Configurable parameters of the `OutQueueControl`
pub(crate) struct Config {
    /// Key used to encrypt and decrypt content of an ACK packet.
    ack_key: Arc<AckKey>,

    /// Represents full address of this client.
    our_full_destination: Recipient,

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
        ack_key: Arc<AckKey>,
        our_full_destination: Recipient,
        average_ack_delay: Duration,
        average_packet_delay: Duration,
        average_message_sending_delay: Duration,
        disable_poisson_packet_distribution: bool,
    ) -> Self {
        Config {
            ack_key,
            our_full_destination,
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
    #[cfg(not(target_arch = "wasm32"))]
    next_delay: Option<Pin<Box<time::Sleep>>>,

    #[cfg(target_arch = "wasm32")]
    next_delay: Option<Pin<Box<wasm_timer::Delay>>>,

    // To make sure we don't overload the mix_tx channel, we limit the rate we are pushing
    // messages.
    sending_delay_controller: SendingDelayController,

    /// Channel used for sending prepared sphinx packets to `MixTrafficController` that sends them
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
}

#[derive(Debug)]
pub(crate) struct RealMessage {
    mix_packet: MixPacket,
    fragment_id: FragmentIdentifier,
    // TODO: add info about it being constructed with reply-surb
}

impl From<PreparedFragment> for RealMessage {
    fn from(fragment: PreparedFragment) -> Self {
        RealMessage {
            mix_packet: fragment.mix_packet,
            fragment_id: fragment.fragment_identifier,
        }
    }
}

impl RealMessage {
    pub(crate) fn packet_size(&self) -> usize {
        self.mix_packet.sphinx_packet().len()
    }

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
    tokio::sync::mpsc::Sender<(Vec<RealMessage>, TransmissionLane)>;
type BatchRealMessageReceiver = tokio::sync::mpsc::Receiver<(Vec<RealMessage>, TransmissionLane)>;

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
        rng: R,
        sent_notifier: SentPacketNotificationSender,
        mix_tx: BatchMixMessageSender,
        real_receiver: BatchRealMessageReceiver,
        topology_access: TopologyAccessor,
        lane_queue_lengths: LaneQueueLengths,
        client_connection_rx: ConnectionCommandReceiver,
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
        }
    }

    fn sent_notify(&self, frag_id: FragmentIdentifier) {
        // well technically the message was not sent just yet, but now it's up to internal
        // queues and client load rather than the required delay. So realistically we can treat
        // whatever is about to happen as negligible additional delay.
        trace!("{} is about to get sent to the mixnet", frag_id);
        self.sent_notifier.unbounded_send(frag_id).unwrap();
    }

    async fn craft_dummy_packet(&mut self) -> Option<MixPacket> {
        let topology_permit = self.topology_access.get_read_permit().await;
        // the ack is sent back to ourselves (and then ignored)
        let topology_ref = match topology_permit.try_get_valid_topology_ref(
            &self.config.our_full_destination,
            Some(&self.config.our_full_destination),
        ) {
            Ok(topology) => topology,
            Err(err) => {
                warn!("We're not going to send any loop cover message this time, as the current topology seem to be invalid - {err}");
                return None;
            }
        };
        Some(generate_loop_cover_packet(
            &mut self.rng,
            topology_ref,
            &*self.config.ack_key,
            &self.config.our_full_destination,
            self.config.average_ack_delay,
            self.config.average_packet_delay,
            self.config.cover_packet_size,
        )
        .expect("Somehow failed to generate a loop cover message with a valid topology"))
    }

    async fn on_message(&mut self, next_message: StreamMessage, dummy_packet: Option<MixPacket>) {
        trace!("created new message");

        let (next_message, fragment_id) = match next_message {
            StreamMessage::Cover => {
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
                println!("cover sent");
                match dummy_packet {
                    Some(packet) => (packet, None),
                    None => 
                    (
                        generate_loop_cover_packet(
                            &mut self.rng,
                            topology_ref,
                            &self.config.ack_key,
                            &self.config.our_full_destination,
                            self.config.average_ack_delay,
                            self.config.average_packet_delay,
                            self.config.cover_packet_size,
                        )
                        .expect("Somehow failed to generate a loop cover message with a valid topology"),
                        None,
                    )
                }
            }
            StreamMessage::Real(real_message) => {
                let time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
                println!("real sent: /{:?}/{}", real_message.fragment_id, time);
                (real_message.mix_packet, Some(real_message.fragment_id))

            }
        };

        if let Err(err) = self.mix_tx.send(vec![next_message]).await {
            log::error!("Failed to send: {err}");
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
        self.config.average_message_sending_delay
            * self.sending_delay_controller.current_multiplier()
    }

    fn adjust_current_average_message_sending_delay(&mut self) {
        let used_slots = self.mix_tx.max_capacity() - self.mix_tx.capacity();
        log::trace!(
            "used_slots: {used_slots}, current_multiplier: {}",
            self.sending_delay_controller.current_multiplier()
        );

        // Even just a single used slot is enough to signal backpressure
        if used_slots > 0 {
            log::trace!("Backpressure detected");
            self.sending_delay_controller.record_backpressure_detected();
        }

        // If the buffer is running out, slow down the sending rate
        if self.mix_tx.capacity() == 0
            && self.sending_delay_controller.not_increased_delay_recently()
        {
            self.sending_delay_controller.increase_delay_multiplier();
        }

        // Very carefully step up the sending rate in case it seems like we can solidly handle the
        // current rate.
        if self.sending_delay_controller.is_sending_reliable() {
            self.sending_delay_controller.decrease_delay_multiplier();
        }
    }

    fn pop_next_message(&mut self) -> Option<RealMessage> {
        // Pop the next message from the transmission buffer
        let (lane, real_next) = self
            .transmission_buffer
            .pop_next_message_at_random(&mut self.rng)?;

        // Update the published queue length
        let lane_length = self.transmission_buffer.lane_length(&lane);
        self.lane_queue_lengths.set(&lane, lane_length);

        Some(real_next)
    }

    fn poll_poisson(&mut self, cx: &mut Context<'_>) -> Poll<Option<StreamMessage>> {
        // The average delay could change depending on if backpressure in the downstream channel
        // (mix_tx) was detected.
        //self.adjust_current_average_message_sending_delay();
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

            // On every iteration we get new messages from upstream. Given that these come bunched
            // in `Vec`, this ensures that on average we will fetch messages faster than we can
            // send, which is a condition for being able to multiplex sphinx packets from multiple
            // data streams.
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
        if self.config.disable_poisson_packet_distribution {
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
        let status_str = if self.config.disable_poisson_packet_distribution {
            format!("Status: {lanes} lanes, backlog: {backlog:.2} kiB ({packets}), no delay")
        } else {
            format!(
                "Status: {lanes} lanes, backlog: {backlog:.2} kiB ({packets}), avg delay: {delay}ms ({mult})"
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

    #[cfg(not(target_arch = "wasm32"))]
    fn log_status_infrequent(&self) {
        if self.sending_delay_controller.current_multiplier() > 1 {
            log::warn!(
                "Unable to send packets at the default rate - rate reduced by setting the delay multiplier set to: {}",
                self.sending_delay_controller.current_multiplier()
            );
        }
    }

    pub(super) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        self.run_test().await;
        return;
        println!("START LINE");
        debug!("Started OutQueueControl with graceful shutdown support");

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut status_timer = tokio::time::interval(Duration::from_secs(5));
            let mut infrequent_status_timer = tokio::time::interval(Duration::from_secs(60));

            while !shutdown.is_shutdown() {
                tokio::select! {
                    biased;
                    _ = shutdown.recv_with_delay() => {
                        log::trace!("OutQueueControl: Received shutdown");
                    }
                    _ = status_timer.tick() => {
                        self.log_status(&mut shutdown);
                    }
                    _ = infrequent_status_timer.tick() => {
                        self.log_status_infrequent();
                    }
                    next_message = self.next() => if let Some(next_message) = next_message {
                        self.on_message(next_message, None).await;
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
        assert!(shutdown.is_shutdown_poll());
        log::debug!("OutQueueControl: Exiting");
    }

    pub(super) async fn run_test(&mut self) {
        warn!("Started OutQueueControl in test mode");
        warn!("Using packets of {:?} bytes",self.config.cover_packet_size.size());
        let dummy_packet = self.craft_dummy_packet().await.unwrap().into_bytes();

        sleep(Duration::new(5, 0));
        info!("Starting warmup phase");

        let mut now = Instant::now(); 
        while let Some(next_message) = self.next().await {
           let packet = MixPacket::try_from_bytes(&dummy_packet.clone()).unwrap();
           self.on_message(next_message, Some(packet)).await;
           if now.elapsed().as_secs() > 10 {
               break;
           }
        }

        info!("10sec warmup done");
        sleep(Duration::new(10, 0));
        info!("10 seconds cooldown elapsed");
        info!("Resetting delay");
        self.next_delay = None;

        info!("Starting measurement");
        println!("START LINE");

        now = Instant::now(); 
        while let Some(next_message) = self.next().await {
            let packet = MixPacket::try_from_bytes(&dummy_packet.clone()).unwrap();
            self.on_message(next_message, Some(packet)).await;
            if now.elapsed().as_secs() > 300 {
                break;
            }
        }
        println!("STOP LINE");
        info!("Stopping stream after 5min");
        sleep(Duration::new(10, 0));
        info!("10 seconds cooldown elapsed");

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
