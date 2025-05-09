// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::mixnet::handler::ConnectionHandler;
use crate::node::mixnet::SharedFinalHopData;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use nym_gateway::node::GatewayStorageError;
use nym_mixnet_client::forwarder::{MixForwardingSender, PacketToForward};
use nym_node_metrics::mixnet::PacketKind;
use nym_node_metrics::NymNodeMetrics;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::processing::{
    MixPacketVersion, MixProcessingResult, MixProcessingResultData, PacketProcessingError,
};
use nym_sphinx_types::DestinationAddressBytes;
use nym_task::ShutdownToken;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{debug, error};

pub(crate) mod final_hop;

#[derive(Clone, Copy)]
pub(crate) struct ProcessingConfig {
    pub(crate) maximum_packet_delay: Duration,
    /// how long the task is willing to skip mutex acquisition before it will block the thread
    /// until it actually obtains it
    pub(crate) maximum_replay_detection_deferral: Duration,

    /// how many packets the task is willing to queue before it will block the thread
    /// until it obtains the mutex
    pub(crate) maximum_replay_detection_pending_packets: usize,

    pub(crate) forward_hop_processing_enabled: bool,
    pub(crate) final_hop_processing_enabled: bool,
}

impl ProcessingConfig {
    pub(crate) fn new(config: &Config) -> Self {
        ProcessingConfig {
            maximum_packet_delay: config.mixnet.debug.maximum_forward_packet_delay,
            maximum_replay_detection_deferral: config
                .mixnet
                .replay_protection
                .debug
                .maximum_replay_detection_deferral,
            maximum_replay_detection_pending_packets: config
                .mixnet
                .replay_protection
                .debug
                .maximum_replay_detection_pending_packets,
            forward_hop_processing_enabled: config.modes.mixnode,
            final_hop_processing_enabled: config.modes.expects_final_hop_traffic()
                || config.wireguard.enabled,
        }
    }
}

// explicitly do NOT derive clone as we want to manually apply relevant suffixes to the task clients
pub(crate) struct SharedData {
    pub(super) processing_config: ProcessingConfig,
    pub(super) sphinx_keys: ActiveSphinxKeys,
    pub(super) replay_protection_filter: ReplayProtectionBloomfilters,

    // used for FORWARD mix packets and FINAL ack packets
    pub(super) mixnet_forwarder: MixForwardingSender,

    // data specific to the final hop (gateway) processing
    pub(super) final_hop: SharedFinalHopData,

    pub(super) metrics: NymNodeMetrics,
    pub(super) shutdown: ShutdownToken,
}

fn convert_to_metrics_version(processed: MixPacketVersion) -> PacketKind {
    match processed {
        MixPacketVersion::Outfox => PacketKind::Outfox,
        MixPacketVersion::Sphinx(sphinx_version) => PacketKind::Sphinx(sphinx_version.value()),
    }
}

impl SharedData {
    pub(crate) fn new(
        processing_config: ProcessingConfig,
        sphinx_keys: ActiveSphinxKeys,
        replay_protection_filter: ReplayProtectionBloomfilters,
        mixnet_forwarder: MixForwardingSender,
        final_hop: SharedFinalHopData,
        metrics: NymNodeMetrics,
        shutdown: ShutdownToken,
    ) -> Self {
        SharedData {
            processing_config,
            sphinx_keys,
            replay_protection_filter,
            mixnet_forwarder,
            final_hop,
            metrics,
            shutdown,
        }
    }

    pub(super) fn log_connected_clients(&self) {
        debug!(
            "there are currently {} connected clients on the mixnet socket",
            self.metrics
                .network
                .active_ingress_mixnet_connections_count()
        )
    }

    pub(super) fn dropped_forward_packet(&self, source: IpAddr) {
        self.metrics.mixnet.ingress_dropped_forward_packet(source)
    }

    pub(super) fn dropped_final_hop_packet(&self, source: IpAddr) {
        self.metrics.mixnet.ingress_dropped_final_hop_packet(source)
    }

    pub(super) fn update_metrics(
        &self,
        processing_result: &Result<MixProcessingResult, PacketProcessingError>,
        source: IpAddr,
    ) {
        let Ok(processing_result) = processing_result else {
            self.metrics.mixnet.ingress_malformed_packet(source);
            return;
        };

        let packet_version = convert_to_metrics_version(processing_result.packet_version);

        match processing_result.processing_data {
            MixProcessingResultData::ForwardHop { delay, .. } => {
                self.metrics
                    .mixnet
                    .ingress_received_forward_packet(source, packet_version);

                // check if the delay wasn't excessive
                if let Some(delay) = delay {
                    if delay.to_duration() > self.processing_config.maximum_packet_delay {
                        self.metrics.mixnet.ingress_excessive_delay_packet()
                    }
                }
            }
            MixProcessingResultData::FinalHop { .. } => {
                self.metrics
                    .mixnet
                    .ingress_received_final_hop_packet(source, packet_version);
            }
        }
    }

    pub(super) fn try_handle_connection(
        &self,
        accepted: io::Result<(TcpStream, SocketAddr)>,
    ) -> Option<JoinHandle<()>> {
        match accepted {
            Ok((socket, remote_addr)) => {
                debug!("accepted incoming mixnet connection from: {remote_addr}");
                let mut handler = ConnectionHandler::new(self, socket, remote_addr);
                let join_handle = tokio::spawn(async move { handler.handle_stream().await });
                self.log_connected_clients();
                Some(join_handle)
            }
            Err(err) => {
                debug!("failed to accept incoming mixnet connection: {err}");
                None
            }
        }
    }

    pub(super) fn forward_mix_packet(&self, packet: MixPacket, delay_until: Option<Instant>) {
        if self
            .mixnet_forwarder
            .forward_packet(PacketToForward::new(packet, delay_until))
            .is_err()
            && !self.shutdown.is_cancelled()
        {
            error!("failed to forward sphinx packet on the channel while the process is not going through the shutdown!");
            self.shutdown.cancel();
        }
    }

    pub(super) fn forward_ack_packet(&self, forward_ack: Option<MixPacket>) {
        if let Some(forward_ack) = forward_ack {
            self.forward_mix_packet(forward_ack, None);
            self.metrics.mixnet.egress_sent_ack();
        }
    }

    pub(super) fn try_push_message_to_client(
        &self,
        client: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), Vec<u8>> {
        self.final_hop.try_push_message_to_client(client, message)
    }

    pub(crate) async fn store_processed_packet_payload(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), GatewayStorageError> {
        self.final_hop
            .store_processed_packet_payload(client_address, message)
            .await
    }
}
