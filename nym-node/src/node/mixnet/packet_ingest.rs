//! Mix Packet Ingest Worker implementation and tooling

use std::io;
use std::net::SocketAddr;
use std::time::Instant;

use crate::node::{
    key_rotation::active_keys::SphinxKeyGuard,
    mixnet::{SharedData, shared::ProcessingConfig},
};
use nym_node_metrics::mixnet::PacketKind;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_framing::processing::{
    MixPacketVersion, MixProcessingResult, MixProcessingResultData, PacketProcessingError,
    PartiallyUnwrappedPacket, PartiallyUnwrappedPacketWithKeyRotation, ProcessedFinalHop,
    process_framed_packet,
};
use nym_sphinx_params::SphinxKeyRotation;
use nym_sphinx_types::Delay;
use nym_task::ShutdownToken;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tracing::{Span, debug, error, instrument, trace, warn};

pub(crate) struct IngressNymPacket {
    packet: FramedNymPacket,
    received_at: Instant,
    received_from: SocketAddr,
}

impl IngressNymPacket {
    pub(crate) fn new(
        packet: FramedNymPacket,
        received_at: Instant,
        received_from: SocketAddr,
    ) -> Self {
        Self {
            packet,
            received_at,
            received_from,
        }
    }
}

pub struct MixPacketIngest {
    shared: SharedData,

    packet_sender: MixIngestSender,
    packet_receiver: MixIngestReceiver,
}

impl MixPacketIngest {
    pub fn new(shared: &SharedData) -> Self {
        let (packet_sender, packet_receiver) = mix_ingest_channels(&shared.processing_config);

        Self {
            shared: SharedData {
                processing_config: shared.processing_config,
                sphinx_keys: shared.sphinx_keys.clone(),
                replay_protection_filter: shared.replay_protection_filter.clone(),
                mixnet_forwarder: shared.mixnet_forwarder.clone(),
                final_hop: shared.final_hop.clone(),
                noise_config: shared.noise_config.clone(),
                metrics: shared.metrics.clone(),
                shutdown_token: shared.shutdown_token.child_token(),
            },

            packet_sender,
            packet_receiver,
        }
    }

    pub fn sender(&self) -> MixIngestSender {
        self.packet_sender.clone()
    }

    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        trace!("starting PacketIngest");
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    debug!("PacketIngest: Received shutdown");
                    break;
                }
                new_packet = self.packet_receiver.recv() => {
                    let Some(new_packet) = new_packet else {
                        todo!("the ingest receiver closed somehow")
                    };
                    self.handle_ingest_packet(new_packet).await;
                }
            }
        }
    }

    async fn handle_ingest_packet(&mut self, packet: IngressNymPacket) {
        // 1. attempt to unwrap the packet
        // if it's a sphinx packet attempt to do pre-processing and replay detection
        if packet.packet.is_sphinx() && !self.shared.replay_protection_filter.disabled() {
            self.handle_received_packet_with_replay_detection(packet)
                .await;
        } else {
            // otherwise just skip that whole procedure and go straight to payload unwrapping
            // (assuming the basic framing is valid)
            self.handle_received_packet_with_no_replay_detection(packet)
                .await;
        };
    }

    async fn handle_received_packet_with_replay_detection(&mut self, packet: IngressNymPacket) {
        let source = packet.received_from.clone();
        let received_at = packet.received_at;

        // 1. derive and expand shared secret
        // also check the header integrity
        let partially_unwrapped = match self.try_partially_unwrap_packet(packet) {
            Ok(unwrapped) => unwrapped,
            Err(err) => {
                trace!("failed to process received mix packet: {err}");
                warn!(
                    event = "packet.dropped.malformed",
                    error = %err,
                    remote_addr = %source,
                    "dropping malformed packet"
                );
                self.shared
                    .metrics
                    .mixnet
                    .ingress_malformed_packet(source.ip());
                return;
            }
        };

        let rotation_id = partially_unwrapped.used_key_rotation;
        let Some(replay_tag) = partially_unwrapped.packet.replay_tag() else {
            error!("corrupted packet - replay tag was missing");
            return;
        };

        let mutex_start = Instant::now();
        let Ok(replayed) = self
            .shared
            .replay_protection_filter
            .check_and_set(rotation_id, &replay_tag)
        else {
            // our mutex got poisoned - we have to shut down
            error!("CRITICAL FAILURE: replay bloomfilter mutex poisoning!");
            self.shared.shutdown_token.cancel();
            return;
        };
        Span::current().record("mutex_wait_ms", mutex_start.elapsed().as_millis() as u64);

        let unwrapped_packet = if replayed {
            warn!(
                event = "packet.dropped.replay",
                remote_addr = %source,
                rotation_id,
                "dropping replayed packet"
            );
            Err(PacketProcessingError::PacketReplay)
        } else {
            partially_unwrapped.packet.finalise_unwrapping()
        };

        self.handle_unwrapped_packet(unwrapped_packet, source, received_at)
            .await;
    }

    async fn handle_received_packet_with_no_replay_detection(&mut self, packet: IngressNymPacket) {
        let source = packet.received_from.clone();
        let received_at = packet.received_at;
        let unwrapped_packet = self.try_full_unwrap_packet(packet);
        self.handle_unwrapped_packet(unwrapped_packet, source, received_at)
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
        packet: IngressNymPacket,
    ) -> Result<MixProcessingResult, PacketProcessingError> {
        let key = self.resolve_rotation_key(packet.packet.header().key_rotation)?;
        process_framed_packet(packet.packet, key.inner().as_ref())
    }

    #[instrument(
        name = "mixnode.sphinx_partial_unwrap",
        skip(self, packet),
        level = "debug",
        fields(key_rotation, unwrap_result,)
    )]
    fn try_partially_unwrap_packet(
        &self,
        packet: IngressNymPacket,
    ) -> Result<PartiallyUnwrappedPacketWithKeyRotation, PacketProcessingError> {
        let rotation = packet.packet.header().key_rotation;

        let result = match rotation {
            SphinxKeyRotation::Unknown => {
                // Unknown rotation: try primary, fallback to secondary
                let primary = self.resolve_rotation_key(rotation)?;
                let primary_rotation = primary.rotation_id();

                match PartiallyUnwrappedPacket::new(packet.packet, primary.inner().as_ref()) {
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
                PartiallyUnwrappedPacket::new(packet.packet, key.inner().as_ref())
                    .map_err(|(_, err)| err)
                    .map(|p| p.with_key_rotation(rotation_id))
            }
        };

        Span::current().record("unwrap_result", if result.is_ok() { "ok" } else { "err" });
        result
    }

    async fn handle_unwrapped_packet(
        &self,
        unwrapped_packet: Result<MixProcessingResult, PacketProcessingError>,
        source: SocketAddr,
        received_at: Instant,
    ) {
        // 2. increment our favourite metrics stats
        self.update_metrics(&unwrapped_packet, source);

        // 3. forward the packet to the relevant sink (if enabled)
        match unwrapped_packet {
            Err(err) => {
                trace!("failed to process received mix packet: {err}");
            }
            Ok(processed_packet) => match processed_packet.processing_data {
                MixProcessingResultData::ForwardHop { packet, delay } => {
                    self.handle_forward_packet(packet, received_at, source, delay);
                }
                MixProcessingResultData::FinalHop { final_hop_data } => {
                    self.handle_final_hop(final_hop_data, source).await;
                }
            },
        }
    }

    fn update_metrics(
        &self,
        processing_result: &Result<MixProcessingResult, PacketProcessingError>,
        source: SocketAddr,
    ) {
        let Ok(processing_result) = processing_result else {
            self.shared
                .metrics
                .mixnet
                .ingress_malformed_packet(source.ip());
            return;
        };

        let packet_version = convert_to_metrics_version(processing_result.packet_version);

        match processing_result.processing_data {
            MixProcessingResultData::ForwardHop { delay, .. } => {
                self.shared
                    .metrics
                    .mixnet
                    .ingress_received_forward_packet(source.ip(), packet_version);

                // check if the delay wasn't excessive
                if let Some(delay) = delay
                    && delay.to_duration() > self.shared.processing_config.maximum_packet_delay
                {
                    self.shared.metrics.mixnet.ingress_excessive_delay_packet()
                }
            }
            MixProcessingResultData::FinalHop { .. } => {
                self.shared
                    .metrics
                    .mixnet
                    .ingress_received_final_hop_packet(source.ip(), packet_version);
            }
        }
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
                    "dropping packet: odd key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
            SphinxKeyRotation::EvenRotation => self.shared.sphinx_keys.even().ok_or_else(|| {
                warn!(
                    event = "packet.dropped.expired_key",
                    key_rotation = "even",
                    "dropping packet: even key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
        }
    }

    #[instrument(
        name = "mixnode.forward_packet",
        skip(self, mix_packet, delay),
        level = "debug",
        fields(
            remote_addr = %remote_addr,
            delay_ms = tracing::field::Empty,
        )
    )]
    fn handle_forward_packet(
        &self,
        mix_packet: MixPacket,
        received_at: Instant,
        remote_addr: SocketAddr,
        delay: Option<Delay>,
    ) {
        if !self.shared.processing_config.forward_hop_processing_enabled {
            warn!(
                event = "packet.dropped.forward_disabled",
                remote_addr = %remote_addr,
                "dropping packet: forward hop processing disabled"
            );
            self.shared.dropped_forward_packet(remote_addr.ip());
            return;
        }

        let forward_instant = self.create_delay_target(received_at, delay);
        if let Some(target) = forward_instant {
            Span::current().record(
                "delay_ms",
                target.saturating_duration_since(received_at).as_millis() as u64,
            );
        }
        self.shared
            .forward_mix_packet(mix_packet, forward_instant.into());
    }

    /// Determine instant at which packet should get forwarded to the next hop.
    /// By using [`Instant`] rather than explicit [`Duration`] we minimise effects of
    /// the skew caused by being stuck in the channel queue.
    /// This method also clamps the maximum allowed delay so that nobody could send a bunch of packets
    /// with, for example, delays of 1 year thus causing denial of service
    fn create_delay_target(&self, received_at: Instant, delay: Option<Delay>) -> Option<Instant> {
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

        Some(received_at + delay)
    }

    #[instrument(
        name = "mixnode.final_hop",
        skip(self, final_hop_data),
        level = "debug",
        fields(
            remote_addr = %remote_addr,
            client_online,
            disk_fallback = false,
            ack_forwarded = false,
        )
    )]
    async fn handle_final_hop(&self, final_hop_data: ProcessedFinalHop, remote_addr: SocketAddr) {
        if !self.shared.processing_config.final_hop_processing_enabled {
            warn!(
                event = "packet.dropped.final_hop_disabled",
                remote_addr = %remote_addr,
                "dropping packet: final hop processing disabled"
            );
            self.shared.dropped_final_hop_packet(remote_addr.ip());
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
}

fn convert_to_metrics_version(processed: MixPacketVersion) -> PacketKind {
    match processed {
        MixPacketVersion::Outfox => PacketKind::Outfox,
        MixPacketVersion::Sphinx(sphinx_version) => PacketKind::Sphinx(sphinx_version.value()),
    }
}

pub fn mix_ingest_channels(
    processing_config: &ProcessingConfig,
) -> (MixIngestSender, MixIngestReceiver) {
    let (tx, rx) = mpsc::channel(processing_config.ingress_channel_maximum_capacity);
    (MixIngestSender(tx), rx)
}

#[derive(Clone)]
pub struct MixIngestSender(mpsc::Sender<IngressNymPacket>);

impl MixIngestSender {
    pub fn ingest_packet(&self, packet: impl Into<IngressNymPacket>) -> io::Result<()> {
        let sender = &self.0;

        let channel_capacity = sender.max_capacity();
        let channel_available = sender.capacity();
        let channel_used = channel_capacity - channel_available;

        let sending_res = sender.try_send(packet.into());

        sending_res.map_err(|err| match err {
            TrySendError::Full(_) => {
                warn!(
                    event = "mixnode.ingress_try_send",
                    result = "full_dropped",
                    channel_capacity,
                    channel_used,
                    "dropping packet: ingress buffer is full ({channel_used}/{channel_capacity})"
                );
                io::Error::new(io::ErrorKind::WouldBlock, "ingress queue is full")
            }
            TrySendError::Closed(_) => {
                debug!(
                    event = "mixnode.ingress_try_send",
                    result = "closed",
                    channel_capacity,
                    channel_used,
                    "ingress queue is closed"
                );
                io::Error::new(io::ErrorKind::BrokenPipe, "ingress packet channel closed")
            }
        })
    }
}

pub type MixIngestReceiver = mpsc::Receiver<IngressNymPacket>;
