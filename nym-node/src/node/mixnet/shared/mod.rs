// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node::mixnet::handler::ConnectionHandler;
use crate::node::mixnet::SharedFinalHopData;
use nym_crypto::asymmetric::x25519;
use nym_gateway::node::GatewayStorageError;
use nym_mixnet_client::forwarder::{MixForwardingSender, PacketToForward};
use nym_node_metrics::NymNodeMetrics;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::processing::{MixProcessingResult, PacketProcessingError};
use nym_sphinx_types::DestinationAddressBytes;
use nym_task::TaskClient;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{debug, error};

pub(crate) mod final_hop;

#[derive(Clone, Copy)]
pub(crate) struct ProcessingConfig {
    pub(crate) maximum_packet_delay: Duration,

    pub(crate) forward_hop_processing_enabled: bool,
    pub(crate) final_hop_processing_enabled: bool,
}

impl ProcessingConfig {
    pub(crate) fn new(config: &Config) -> Self {
        ProcessingConfig {
            maximum_packet_delay: config.mixnet.debug.maximum_forward_packet_delay,
            forward_hop_processing_enabled: config.modes.mixnode,
            final_hop_processing_enabled: config.modes.expects_final_hop_traffic(),
        }
    }
}

// explicitly do NOT derive clone as we want to manually apply relevant suffixes to the task clients
// as well as immediately disarm them
pub(crate) struct SharedData {
    pub(super) processing_config: ProcessingConfig,
    // TODO: this type is not `Zeroize` : (
    pub(super) sphinx_key: Arc<nym_sphinx_types::PrivateKey>,

    // used for FORWARD mix packets and FINAL ack packets
    pub(super) mixnet_forwarder: MixForwardingSender,

    // data specific to the final hop (gateway) processing
    pub(super) final_hop: SharedFinalHopData,

    pub(super) metrics: NymNodeMetrics,
    pub(super) task_client: TaskClient,
}

impl SharedData {
    pub(crate) fn new(
        config: &Config,
        x25519_key: &x25519::PrivateKey,
        mixnet_forwarder: MixForwardingSender,
        final_hop: SharedFinalHopData,
        metrics: NymNodeMetrics,
        task_client: TaskClient,
    ) -> Self {
        SharedData {
            processing_config: ProcessingConfig::new(config),
            sphinx_key: Arc::new(x25519_key.into()),
            mixnet_forwarder,
            final_hop,
            metrics,
            task_client,
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
        match processing_result {
            Err(_) => self.metrics.mixnet.ingress_malformed_packet(source),
            Ok(MixProcessingResult::ForwardHop(_, delay)) => {
                self.metrics.mixnet.ingress_received_forward_packet(source);

                // check if the delay wasn't excessive
                if let Some(delay) = delay {
                    if delay.to_duration() > self.processing_config.maximum_packet_delay {
                        self.metrics.mixnet.ingress_excessive_delay_packet()
                    }
                }
            }
            Ok(MixProcessingResult::FinalHop(_)) => {
                self.metrics
                    .mixnet
                    .ingress_received_final_hop_packet(source);
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
            && !self.task_client.is_shutdown()
        {
            error!("failed to forward sphinx packet on the channel while the process is not going through the shutdown!");
            // this is a critical error, we're in uncharted lands, we have to shut down
            let mut shutdown_bomb = self.task_client.fork("shutdown bomb");
            shutdown_bomb.rearm();
            drop(shutdown_bomb)
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
