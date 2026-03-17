// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::key_rotation::active_keys::SphinxKeyGuard;
use crate::node::mixnet::shared::SharedData;
use futures::StreamExt;
use nym_noise::connection::Connection;
use nym_noise::upgrade_noise_responder;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::codec::NymCodec;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_framing::processing::{
    MixProcessingResult, MixProcessingResultData, PacketProcessingError, PartiallyUnwrappedPacket,
    PartialyUnwrappedPacketWithKeyRotation, ProcessedFinalHop, process_framed_packet,
};
use nym_sphinx_params::SphinxKeyRotation;
use nym_sphinx_types::{Delay, REPLAY_TAG_SIZE};
use std::collections::HashMap;
use std::mem;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;
use tracing::{Span, debug, error, instrument, trace, warn};

/// How often (in packets) the stream-level span updates its packet count.
const SPAN_UPDATE_INTERVAL: u64 = 10_000;

struct PendingReplayCheckPackets {
    // map of rotation id used for packet creation to the packets
    packets: HashMap<u32, Vec<PartiallyUnwrappedPacket>>,
    last_acquired_mutex: Instant,
}

impl PendingReplayCheckPackets {
    fn new() -> PendingReplayCheckPackets {
        PendingReplayCheckPackets {
            packets: Default::default(),
            last_acquired_mutex: Instant::now(),
        }
    }

    fn reset(&mut self, now: Instant) -> HashMap<u32, Vec<PartiallyUnwrappedPacket>> {
        self.last_acquired_mutex = now;
        mem::take(&mut self.packets)
    }

    fn push(&mut self, now: Instant, packet: PartialyUnwrappedPacketWithKeyRotation) {
        if self.packets.is_empty() {
            self.last_acquired_mutex = now;
        }
        self.packets
            .entry(packet.used_key_rotation)
            .or_default()
            .push(packet.packet)
    }

    fn total_count(&self) -> usize {
        self.packets.values().map(|v| v.len()).sum()
    }

    fn replay_tags(&self) -> HashMap<u32, Vec<&[u8; REPLAY_TAG_SIZE]>> {
        let mut replay_tags = HashMap::with_capacity(self.packets.len());
        'outer: for (rotation_id, packets) in &self.packets {
            let mut rotation_replay_tags = Vec::with_capacity(packets.len());
            for packet in packets {
                let Some(replay_tag) = packet.replay_tag() else {
                    error!(
                        "corrupted batch of {} packets - replay tag was missing",
                        self.packets.len()
                    );
                    replay_tags.insert(*rotation_id, Vec::new());
                    continue 'outer;
                };
                rotation_replay_tags.push(replay_tag);
            }
            replay_tags.insert(*rotation_id, rotation_replay_tags);
        }
        replay_tags
    }
}

pub(crate) struct ConnectionHandler {
    shared: SharedData,
    remote_address: SocketAddr,

    // packets pending for replay detection
    pending_packets: PendingReplayCheckPackets,
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        self.shared
            .metrics
            .network
            .disconnected_ingress_mixnet_client()
    }
}

impl ConnectionHandler {
    pub(crate) fn new(shared: &SharedData, remote_address: SocketAddr) -> Self {
        shared.metrics.network.new_active_ingress_mixnet_client();

        ConnectionHandler {
            shared: SharedData {
                processing_config: shared.processing_config,
                sphinx_keys: shared.sphinx_keys.clone(),
                replay_protection_filter: shared.replay_protection_filter.clone(),
                mixnet_forwarder: shared.mixnet_forwarder.clone(),
                final_hop: shared.final_hop.clone(),
                noise_config: shared.noise_config.clone(),
                metrics: shared.metrics.clone(),
                authorised_network_monitor_agents: shared.authorised_network_monitor_agents.clone(),
                shutdown_token: shared.shutdown_token.child_token(),
            },
            remote_address,
            pending_packets: PendingReplayCheckPackets::new(),
        }
    }

    fn is_from_authorised_network_monitor_agent(&self) -> bool {
        self.shared
            .authorised_network_monitor_agents
            .is_known(&self.remote_address.ip())
    }

    /// Determine instant at which packet should get forwarded to the next hop.
    /// By using [`Instant`] rather than explicit [`Duration`], we minimise the effects of
    /// the skew caused by being stuck in the channel queue.
    /// This method also clamps the maximum allowed delay so that nobody could send a bunch of packets
    /// with, for example, delays of 1 year thus causing denial of service
    fn create_delay_target(&self, now: Instant, delay: Option<Delay>) -> Option<Instant> {
        let delay = delay?.to_duration();

        let delay = if delay > self.shared.processing_config.maximum_packet_delay {
            self.shared.processing_config.maximum_packet_delay
        } else {
            delay
        };
        trace!(
            "received packet will be delayed for {}ms",
            delay.as_millis()
        );

        Some(now + delay)
    }

    #[instrument(
        name = "mixnode.forward_packet",
        skip(self, mix_packet, delay),
        level = "debug",
        fields(
            remote_addr = %self.remote_address,
            delay_ms = tracing::field::Empty,
        )
    )]
    fn handle_forward_packet(&self, now: Instant, mix_packet: MixPacket, delay: Option<Delay>) {
        if !self.shared.processing_config.forward_hop_processing_enabled {
            warn!(
                event = "packet.dropped.forward_disabled",
                remote_addr = %self.remote_address,
                "dropping packet: forward hop processing disabled"
            );
            self.shared.dropped_forward_packet(self.remote_address.ip());
            return;
        }

        let forward_instant = self.create_delay_target(now, delay);
        if let Some(target) = forward_instant {
            Span::current().record(
                "delay_ms",
                target.saturating_duration_since(now).as_millis() as u64,
            );
        }
        self.shared.forward_mix_packet(mix_packet, forward_instant);
    }

    #[instrument(
        name = "mixnode.final_hop",
        skip(self, final_hop_data),
        level = "debug",
        fields(
            remote_addr = %self.remote_address,
            client_online,
            disk_fallback = false,
            ack_forwarded = false,
        )
    )]
    async fn handle_final_hop(&self, final_hop_data: ProcessedFinalHop) {
        if !self.shared.processing_config.final_hop_processing_enabled {
            warn!(
                event = "packet.dropped.final_hop_disabled",
                remote_addr = %self.remote_address,
                "dropping packet: final hop processing disabled"
            );
            self.shared
                .dropped_final_hop_packet(self.remote_address.ip());
            return;
        }

        let client = final_hop_data.destination;
        let message = final_hop_data.message;
        let has_ack = final_hop_data.forward_ack.is_some();

        // if possible attempt to push message directly to the client
        match self.shared.try_push_message_to_client(client, message) {
            Err(unsent_plaintext) => {
                // if that failed, store it on disk
                Span::current().record("client_online", false);
                match self
                    .shared
                    .store_processed_packet_payload(client, unsent_plaintext)
                    .await
                {
                    Err(err) => error!("Failed to store client data - {err}"),
                    Ok(_) => {
                        Span::current().record("disk_fallback", true);
                        self.shared
                            .metrics
                            .mixnet
                            .egress
                            .add_disk_persisted_packet();
                        trace!("Stored packet for {client}")
                    }
                }
            }
            Ok(_) => {
                Span::current().record("client_online", true);
                trace!("Pushed received packet to {client}");
            }
        }

        // if we managed to either push message directly to the [online] client or store it at
        // disk, forward the ack
        self.shared.forward_ack_packet(final_hop_data.forward_ack);
        if has_ack {
            Span::current().record("ack_forwarded", true);
        }
    }

    fn within_deferral_threshold(&self, now: Instant) -> bool {
        let time_threshold = now
            .saturating_duration_since(self.pending_packets.last_acquired_mutex)
            <= self
                .shared
                .processing_config
                .maximum_replay_detection_deferral;

        let count_threshold = self.pending_packets.packets.len()
            < self
                .shared
                .processing_config
                .maximum_replay_detection_pending_packets;

        // time threshold is ignored if we currently have 0 packets queued up
        if self.pending_packets.packets.is_empty() {
            return true;
        }

        trace!(
            "within deferral time threshold: {time_threshold}, count threshold: {count_threshold}"
        );

        if !time_threshold {
            warn!(
                event = "replay_detection.deferral_exceeded",
                threshold_type = "time",
                deferred_count = self.pending_packets.total_count(),
                deferral_ms = now.saturating_duration_since(self.pending_packets.last_acquired_mutex).as_millis() as u64,
                remote_addr = %self.remote_address,
                "{}: time deferral threshold exceeded with {} pending packets",
                self.remote_address,
                self.pending_packets.total_count()
            )
        }

        if !count_threshold {
            warn!(
                event = "replay_detection.deferral_exceeded",
                threshold_type = "count",
                deferred_count = self.pending_packets.total_count(),
                remote_addr = %self.remote_address,
                "{}: count deferral threshold exceeded",
                self.remote_address
            )
        }

        time_threshold && count_threshold
    }

    /// Resolve the sphinx key for the given rotation, recording the rotation
    /// label on the current tracing span.  Returns `ExpiredKey` if the requested
    /// odd/even key has already been rotated out.
    fn resolve_rotation_key(
        &self,
        rotation: SphinxKeyRotation,
    ) -> Result<SphinxKeyGuard, PacketProcessingError> {
        let rotation_label = match rotation {
            SphinxKeyRotation::Unknown => "unknown",
            SphinxKeyRotation::OddRotation => "odd",
            SphinxKeyRotation::EvenRotation => "even",
        };
        Span::current().record("key_rotation", rotation_label);

        match rotation {
            SphinxKeyRotation::Unknown => Ok(self.shared.sphinx_keys.primary()),
            SphinxKeyRotation::OddRotation => self.shared.sphinx_keys.odd().ok_or_else(|| {
                warn!(
                    event = "packet.dropped.expired_key",
                    key_rotation = "odd",
                    remote_addr = %self.remote_address,
                    "dropping packet: odd key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
            SphinxKeyRotation::EvenRotation => self.shared.sphinx_keys.even().ok_or_else(|| {
                warn!(
                    event = "packet.dropped.expired_key",
                    key_rotation = "even",
                    remote_addr = %self.remote_address,
                    "dropping packet: even key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
        }
    }

    #[instrument(
        name = "mixnode.sphinx_partial_unwrap",
        skip(self, packet),
        level = "debug",
        fields(key_rotation, unwrap_result,)
    )]
    fn try_partially_unwrap_packet(
        &self,
        packet: FramedNymPacket,
    ) -> Result<PartialyUnwrappedPacketWithKeyRotation, PacketProcessingError> {
        let rotation = packet.header().key_rotation;

        let result = match rotation {
            SphinxKeyRotation::Unknown => {
                // Unknown rotation: try primary, fallback to secondary
                let primary = self.resolve_rotation_key(rotation)?;
                let primary_rotation = primary.rotation_id();

                match PartiallyUnwrappedPacket::new(packet, primary.inner().as_ref()) {
                    Ok(unwrapped_packet) => {
                        Ok(unwrapped_packet.with_key_rotation(primary_rotation))
                    }
                    Err((packet, err)) => {
                        if let Some(secondary) = self.shared.sphinx_keys.secondary() {
                            let secondary_rotation = secondary.rotation_id();
                            PartiallyUnwrappedPacket::new(packet, secondary.inner().as_ref())
                                .map_err(|(_, err)| err)
                                .map(|p| p.with_key_rotation(secondary_rotation))
                        } else {
                            Err(err)
                        }
                    }
                }
            }
            _ => {
                let key = self.resolve_rotation_key(rotation)?;
                let rotation_id = key.rotation_id();
                PartiallyUnwrappedPacket::new(packet, key.inner().as_ref())
                    .map_err(|(_, err)| err)
                    .map(|p| p.with_key_rotation(rotation_id))
            }
        };

        Span::current().record("unwrap_result", if result.is_ok() { "ok" } else { "err" });
        result
    }

    async fn handle_received_packet_with_replay_detection(
        &mut self,
        now: Instant,
        packet: FramedNymPacket,
    ) {
        // 1. derive and expand shared secret
        // also check the header integrity
        let partially_unwrapped = match self.try_partially_unwrap_packet(packet) {
            Ok(unwrapped) => unwrapped,
            Err(err) => {
                trace!("failed to process received mix packet: {err}");
                warn!(
                    event = "packet.dropped.malformed",
                    error = %err,
                    remote_addr = %self.remote_address,
                    "dropping malformed packet"
                );
                self.shared
                    .metrics
                    .mixnet
                    .ingress_malformed_packet(self.remote_address.ip());
                return;
            }
        };

        self.pending_packets.push(now, partially_unwrapped);

        // 2. check for packet replay
        // 2.1 first try it without locking
        if self.handle_pending_packets_batch_no_locking(now).await {
            return;
        }

        // 2.2 if we're within deferral threshold, just leave it queued up for another call
        if self.within_deferral_threshold(now) {
            return;
        }

        // 2.3. otherwise block until we obtain the lock and clear the whole batch
        self.handle_pending_packets_batch(now).await;
    }

    async fn handle_unwrapped_packet(
        &self,
        now: Instant,
        unwrapped_packet: Result<MixProcessingResult, PacketProcessingError>,
    ) {
        // 2. increment our favourite metrics stats
        self.shared
            .update_metrics(&unwrapped_packet, self.remote_address.ip());

        // 3. forward the packet to the relevant sink (if enabled)
        match unwrapped_packet {
            Err(err) => {
                trace!("failed to process received mix packet: {err}");
            }
            Ok(processed_packet) => match processed_packet.processing_data {
                MixProcessingResultData::ForwardHop { packet, delay } => {
                    self.handle_forward_packet(now, packet, delay);
                }
                MixProcessingResultData::FinalHop { final_hop_data } => {
                    self.handle_final_hop(final_hop_data).await;
                }
            },
        }
    }

    async fn handle_post_replay_detection_packets(
        &self,
        now: Instant,
        packets: HashMap<u32, Vec<PartiallyUnwrappedPacket>>,
        replay_check_results: HashMap<u32, Vec<bool>>,
    ) {
        let mut replays_detected: u64 = 0;
        for (rotation_id, packets) in packets {
            let Some(replay_checks) = replay_check_results.get(&rotation_id) else {
                // this should never happen, but if we messed up, and it does, don't panic, just drop the packets
                error!("inconsistent replay check result - no values for rotation {rotation_id}");
                continue;
            };
            for (packet, &replayed) in packets.into_iter().zip(replay_checks) {
                // if the packet has been replayed and is NOT from a known network monitor agent,
                // do not process it any further
                if replayed && !self.is_from_authorised_network_monitor_agent() {
                    replays_detected += 1;
                    warn!(
                        event = "packet.dropped.replay",
                        remote_addr = %self.remote_address,
                        rotation_id,
                        "dropping replayed packet"
                    );
                    self.handle_unwrapped_packet(now, Err(PacketProcessingError::PacketReplay))
                        .await;
                    continue;
                }

                let unwrapped_packet = packet.finalise_unwrapping();
                self.handle_unwrapped_packet(now, unwrapped_packet).await;
            }
        }
        if replays_detected > 0 {
            debug!(
                replays_detected,
                remote_addr = %self.remote_address,
                "replay detection batch completed with replays"
            );
        }
    }

    async fn handle_pending_packets_batch_no_locking(&mut self, now: Instant) -> bool {
        let replay_tags = self.pending_packets.replay_tags();
        if replay_tags.is_empty() {
            return false;
        }

        let replay_check_results = match self
            .shared
            .replay_protection_filter
            .batch_try_check_and_set(&replay_tags)
        {
            None => return false,
            Some(Ok(replay_check_results)) => replay_check_results,
            Some(Err(_)) => {
                // our mutex got poisoned - we have to shut down
                error!("CRITICAL FAILURE: replay bloomfilter mutex poisoning!");
                self.shared.shutdown_token.cancel();
                return false;
            }
        };

        let batch = self.pending_packets.reset(now);
        self.handle_post_replay_detection_packets(now, batch, replay_check_results)
            .await;
        true
    }

    #[instrument(
        name = "mixnode.replay_check_batch",
        skip(self),
        level = "debug",
        fields(batch_size, mutex_wait_ms,)
    )]
    async fn handle_pending_packets_batch(&mut self, now: Instant) {
        let replay_tags = self.pending_packets.replay_tags();
        if replay_tags.is_empty() {
            return;
        }

        let batch_size = self.pending_packets.total_count();
        Span::current().record("batch_size", batch_size as u64);

        let mutex_start = Instant::now();
        let Ok(replay_check_results) = self
            .shared
            .replay_protection_filter
            .batch_check_and_set(&replay_tags)
        else {
            // our mutex got poisoned - we have to shut down
            error!("CRITICAL FAILURE: replay bloomfilter mutex poisoning!");
            self.shared.shutdown_token.cancel();
            return;
        };
        Span::current().record("mutex_wait_ms", mutex_start.elapsed().as_millis() as u64);

        let batch = self.pending_packets.reset(now);
        self.handle_post_replay_detection_packets(now, batch, replay_check_results)
            .await;
    }

    #[instrument(
        name = "mixnode.sphinx_full_unwrap",
        skip(self, packet),
        level = "debug",
        fields(key_rotation)
    )]
    fn try_full_unwrap_packet(
        &self,
        packet: FramedNymPacket,
    ) -> Result<MixProcessingResult, PacketProcessingError> {
        let key = self.resolve_rotation_key(packet.header().key_rotation)?;
        process_framed_packet(packet, key.inner().as_ref())
    }

    async fn handle_received_packet_with_no_replay_detection(
        &mut self,
        now: Instant,
        packet: FramedNymPacket,
    ) {
        let unwrapped_packet = self.try_full_unwrap_packet(packet);
        self.handle_unwrapped_packet(now, unwrapped_packet).await;
    }

    #[instrument(skip(self, packet), level = "debug")]
    async fn handle_received_nym_packet(&mut self, packet: FramedNymPacket) {
        let now = Instant::now();

        // 1. attempt to unwrap the packet
        // if it's a sphinx packet attempt to do pre-processing and replay detection
        if packet.is_sphinx() && !self.shared.replay_protection_filter.disabled() {
            self.handle_received_packet_with_replay_detection(now, packet)
                .await;
        } else {
            // otherwise just skip that whole procedure and go straight to payload unwrapping
            // (assuming the basic framing is valid)
            self.handle_received_packet_with_no_replay_detection(now, packet)
                .await;
        };
    }

    #[instrument(
        name = "mixnode.connection",
        skip(self, socket),
        level = "debug",
        fields(
            remote = %self.remote_address,
            noise_handshake_ms = tracing::field::Empty,
        )
    )]
    pub(crate) async fn handle_connection(&mut self, socket: TcpStream) {
        let handshake_start = Instant::now();
        let noise_stream = match upgrade_noise_responder(socket, &self.shared.noise_config).await {
            Ok(noise_stream) => noise_stream,
            Err(err) => {
                Span::current().record(
                    "noise_handshake_ms",
                    handshake_start.elapsed().as_millis() as u64,
                );
                warn!(
                    event = "connection.failed.noise",
                    remote_addr = %self.remote_address,
                    error = %err,
                    "Noise responder handshake failed"
                );
                return;
            }
        };
        Span::current().record(
            "noise_handshake_ms",
            handshake_start.elapsed().as_millis() as u64,
        );
        debug!(
            "Noise responder handshake completed for {:?}",
            self.remote_address
        );
        self.handle_stream(Framed::new(noise_stream, NymCodec))
            .await
    }

    #[instrument(
        name = "mixnode.stream",
        skip(self, mixnet_connection),
        level = "debug",
        fields(
            remote = %self.remote_address,
            packets_processed = 0u64,
            exit_reason,
        )
    )]
    pub(crate) async fn handle_stream(
        &mut self,
        mut mixnet_connection: Framed<Connection<TcpStream>, NymCodec>,
    ) {
        let mut packets_processed: u64 = 0;
        loop {
            tokio::select! {
                biased;
                _ = self.shared.shutdown_token.cancelled() => {
                    trace!("connection handler: received shutdown");
                    Span::current().record("exit_reason", "shutdown");
                    break
                }
                maybe_framed_nym_packet = mixnet_connection.next() => {
                    match maybe_framed_nym_packet {
                        Some(Ok(packet)) => {
                            self.handle_received_nym_packet(packet).await;
                            packets_processed += 1;
                            if packets_processed.is_multiple_of(SPAN_UPDATE_INTERVAL) {
                                Span::current().record("packets_processed", packets_processed);
                            }
                        }
                        Some(Err(err)) => {
                            warn!(
                                event = "connection.corrupted",
                                remote_addr = %self.remote_address,
                                error = %err,
                                packets_processed,
                                "connection stream corrupted"
                            );
                            Span::current().record("exit_reason", "corrupted");
                            Span::current().record("packets_processed", packets_processed);
                            return
                        }
                        None => {
                            debug!(
                                remote_addr = %self.remote_address,
                                packets_processed,
                                "connection closed by remote"
                            );
                            Span::current().record("exit_reason", "closed_by_remote");
                            Span::current().record("packets_processed", packets_processed);
                            return
                        }
                    }
                }
            }
        }

        Span::current().record("packets_processed", packets_processed);
        debug!("exiting and closing connection");
    }
}
