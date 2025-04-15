// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::shared::SharedData;
use futures::StreamExt;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::codec::NymCodec;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_framing::processing::{
    process_framed_packet, MixProcessingResult, MixProcessingResultData, PacketProcessingError,
    PartiallyUnwrappedPacket, ProcessedFinalHop,
};
use nym_sphinx_params::SphinxKeyRotation;
use nym_sphinx_types::{Delay, REPLAY_TAG_SIZE};
use std::mem;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;
use tracing::{debug, error, instrument, trace, warn};

struct PendingReplayCheckPackets {
    packets: Vec<PartiallyUnwrappedPacket>,
    last_acquired_mutex: Instant,
}

impl PendingReplayCheckPackets {
    fn new() -> PendingReplayCheckPackets {
        PendingReplayCheckPackets {
            packets: vec![],
            last_acquired_mutex: Instant::now(),
        }
    }

    fn reset(&mut self, now: Instant) -> Vec<PartiallyUnwrappedPacket> {
        self.last_acquired_mutex = now;
        mem::take(&mut self.packets)
    }

    fn push(&mut self, now: Instant, packet: PartiallyUnwrappedPacket) {
        if self.packets.is_empty() {
            self.last_acquired_mutex = now;
        }
        self.packets.push(packet);
    }

    fn replay_tags(&self) -> Vec<&[u8; REPLAY_TAG_SIZE]> {
        let mut replay_tags = Vec::with_capacity(self.packets.len());
        for packet in &self.packets {
            let Some(replay_tag) = packet.replay_tag() else {
                error!(
                    "corrupted batch of {} packets - replay tag was missing",
                    self.packets.len()
                );
                return Vec::new();
            };
            replay_tags.push(replay_tag);
        }
        replay_tags
    }
}

pub(crate) struct ConnectionHandler {
    shared: SharedData,
    mixnet_connection: Framed<TcpStream, NymCodec>,
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
    pub(crate) fn new(
        shared: &SharedData,
        tcp_stream: TcpStream,
        remote_address: SocketAddr,
    ) -> Self {
        let shutdown = shared.shutdown.child_token(remote_address.to_string());
        shared.metrics.network.new_active_ingress_mixnet_client();

        ConnectionHandler {
            shared: SharedData {
                processing_config: shared.processing_config,
                sphinx_keys: shared.sphinx_keys.clone(),
                replay_protection_filter: shared.replay_protection_filter.clone(),
                mixnet_forwarder: shared.mixnet_forwarder.clone(),
                final_hop: shared.final_hop.clone(),
                metrics: shared.metrics.clone(),
                shutdown,
            },
            remote_address,
            mixnet_connection: Framed::new(tcp_stream, NymCodec),
            pending_packets: PendingReplayCheckPackets::new(),
        }
    }

    /// Determine instant at which packet should get forwarded to the next hop.
    /// By using [`Instant`] rather than explicit [`Duration`] we minimise effects of
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

    fn handle_forward_packet(&self, now: Instant, mix_packet: MixPacket, delay: Option<Delay>) {
        if !self.shared.processing_config.forward_hop_processing_enabled {
            trace!("this nym-node does not support forward hop packets");
            self.shared.dropped_forward_packet(self.remote_address.ip());
            return;
        }

        let forward_instant = self.create_delay_target(now, delay);
        self.shared.forward_mix_packet(mix_packet, forward_instant);
    }

    async fn handle_final_hop(&self, final_hop_data: ProcessedFinalHop) {
        if !self.shared.processing_config.final_hop_processing_enabled {
            trace!("this nym-node does not support final hop packets");
            self.shared
                .dropped_final_hop_packet(self.remote_address.ip());
            return;
        }

        let client = final_hop_data.destination;
        let message = final_hop_data.message;

        // if possible attempt to push message directly to the client
        match self.shared.try_push_message_to_client(client, message) {
            Err(unsent_plaintext) => {
                // if that failed, store it on disk (to be 🔥 soon...)
                match self
                    .shared
                    .store_processed_packet_payload(client, unsent_plaintext)
                    .await
                {
                    Err(err) => error!("Failed to store client data - {err}"),
                    Ok(_) => {
                        self.shared
                            .metrics
                            .mixnet
                            .egress
                            .add_disk_persisted_packet();
                        trace!("Stored packet for {client}")
                    }
                }
            }
            Ok(_) => trace!("Pushed received packet to {client}"),
        }

        // if we managed to either push message directly to the [online] client or store it at
        // its inbox, it means that it must exist at this gateway, hence we can send the
        // received ack back into the network
        self.shared.forward_ack_packet(final_hop_data.forward_ack);
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
                "{}: time failure - {}",
                self.remote_address,
                self.pending_packets.packets.len()
            )
        }

        if !count_threshold {
            warn!("{}, count failure", self.remote_address)
        }

        time_threshold && count_threshold
    }

    fn try_partially_unwrap_packet(
        &self,
        packet: FramedNymPacket,
    ) -> Result<PartiallyUnwrappedPacket, PacketProcessingError> {
        // based on the received sphinx key rotation information,
        // attempt to choose appropriate key for processing the packet
        match packet.header().key_rotation {
            SphinxKeyRotation::Unknown => {
                // we have to try both keys, start with the primary as it has higher likelihood of being correct
                // if let Ok(partially_unwrapped) = PartiallyUnwrappedPacket::new()
                match PartiallyUnwrappedPacket::new(packet, self.shared.sphinx_keys.primary()) {
                    Ok(unwrapped_packet) => Ok(unwrapped_packet),
                    Err((packet, err)) => {
                        if let Some(secondary) = self.shared.sphinx_keys.secondary() {
                            PartiallyUnwrappedPacket::new(packet, secondary).map_err(|(_, err)| err)
                        } else {
                            Err(err)
                        }
                    }
                }
            }
            SphinxKeyRotation::OddRotation => {
                let Some(odd_key) = self.shared.sphinx_keys.odd() else {
                    return Err(PacketProcessingError::ExpiredKey);
                };
                PartiallyUnwrappedPacket::new(packet, odd_key).map_err(|(_, err)| err)
            }
            SphinxKeyRotation::EvenRotation => {
                let Some(even_key) = self.shared.sphinx_keys.even() else {
                    return Err(PacketProcessingError::ExpiredKey);
                };
                PartiallyUnwrappedPacket::new(packet, even_key).map_err(|(_, err)| err)
            }
        }
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
            Err(err) => trace!("failed to process received mix packet: {err}"),
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
        packets: Vec<PartiallyUnwrappedPacket>,
        replay_check_results: Vec<bool>,
    ) {
        for (packet, replayed) in packets.into_iter().zip(replay_check_results) {
            let unwrapped_packet = if replayed {
                Err(PacketProcessingError::PacketReplay)
            } else {
                packet.finalise_unwrapping()
            };

            self.handle_unwrapped_packet(now, unwrapped_packet).await;
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
                self.shared.shutdown.cancel();
                return false;
            }
        };

        let batch = self.pending_packets.reset(now);
        self.handle_post_replay_detection_packets(now, batch, replay_check_results)
            .await;
        true
    }

    async fn handle_pending_packets_batch(&mut self, now: Instant) {
        let batch = self.pending_packets.reset(now);
        let replay_tags = self.pending_packets.replay_tags();
        if replay_tags.is_empty() {
            return;
        }

        let Ok(replay_check_results) = self
            .shared
            .replay_protection_filter
            .batch_check_and_set(&replay_tags)
        else {
            // our mutex got poisoned - we have to shut down
            error!("CRITICAL FAILURE: replay bloomfilter mutex poisoning!");
            self.shared.shutdown.cancel();
            return;
        };

        self.handle_post_replay_detection_packets(now, batch, replay_check_results)
            .await;
    }

    fn try_full_unwrap_packet(
        &self,
        packet: FramedNymPacket,
    ) -> Result<MixProcessingResult, PacketProcessingError> {
        // based on the received sphinx key rotation information,
        // attempt to choose appropriate key for processing the packet
        // NOTE: due to the function signatures, outfox packets will **only** attempt primary key
        // if no rotation information is available (but that's fine given outfox is not really in use,
        // and by the time we need it, the rotation info should be present)
        match packet.header().key_rotation {
            SphinxKeyRotation::Unknown => {
                process_framed_packet(packet, self.shared.sphinx_keys.primary())
            }
            SphinxKeyRotation::OddRotation => {
                let Some(odd_key) = self.shared.sphinx_keys.odd() else {
                    return Err(PacketProcessingError::ExpiredKey);
                };
                process_framed_packet(packet, odd_key)
            }
            SphinxKeyRotation::EvenRotation => {
                let Some(even_key) = self.shared.sphinx_keys.even() else {
                    return Err(PacketProcessingError::ExpiredKey);
                };
                process_framed_packet(packet, even_key)
            }
        }
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
        skip(self),
        level = "debug",
        fields(
            remote = %self.remote_address
        )
    )]
    pub(crate) async fn handle_stream(&mut self) {
        loop {
            tokio::select! {
                biased;
                _ = self.shared.shutdown.cancelled() => {
                    trace!("connection handler: received shutdown");
                    break
                }
                maybe_framed_nym_packet = self.mixnet_connection.next() => {
                    match maybe_framed_nym_packet {
                        Some(Ok(packet)) => self.handle_received_nym_packet(packet).await,
                        Some(Err(err)) => {
                            debug!("connection got corrupted with: {err}");
                            return
                        }
                        None => {
                            debug!("connection got closed by the remote");
                            return
                        }
                    }
                }
            }
        }

        debug!("exiting and closing connection");
    }
}
