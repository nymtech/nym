// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::mixnet::SharedFinalHopData;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use nym_gateway::node::GatewayStorageError;
use nym_mixnet_client::forwarder::{MixForwardingSender, PacketToForward};
use nym_node_metrics::NymNodeMetrics;
use nym_noise::config::NoiseConfig;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_types::DestinationAddressBytes;
use nym_task::ShutdownToken;
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tracing::{debug, error};

pub(crate) mod final_hop;

#[derive(Clone, Copy)]
pub(crate) struct ProcessingConfig {
    pub(crate) maximum_packet_delay: Duration,

    /// Channel capacity for the mixnet ingress channel. This determines the maximum number of
    /// packets that can be queued waiting for ingest processing. Once the queue is full packets
    /// will still be taken off the wire, but dropped as the node is too busy to handle them.
    pub(crate) ingress_channel_maximum_capacity: usize,

    pub(crate) forward_hop_processing_enabled: bool,
    pub(crate) final_hop_processing_enabled: bool,
}

impl ProcessingConfig {
    pub(crate) fn new(config: &Config) -> Self {
        ProcessingConfig {
            maximum_packet_delay: config.mixnet.debug.maximum_forward_packet_delay,
            ingress_channel_maximum_capacity: config
                .mixnet
                .replay_protection
                .debug
                .ingress_channel_maximum_capacity,
            forward_hop_processing_enabled: config.modes.mixnode,
            final_hop_processing_enabled: config.modes.expects_final_hop_traffic()
                || config.wireguard.enabled,
        }
    }
}

// explicitly do NOT derive clone as we want the childs to use CHILD shutdown tokens
pub(crate) struct SharedData {
    pub(super) processing_config: ProcessingConfig,
    pub(super) sphinx_keys: ActiveSphinxKeys,
    pub(super) replay_protection_filter: ReplayProtectionBloomfilters,

    // used for FORWARD mix packets and FINAL ack packets
    pub(super) mixnet_forwarder: MixForwardingSender,

    // data specific to the final hop (gateway) processing
    pub(super) final_hop: SharedFinalHopData,

    // for establishing a Noise connection
    pub(super) noise_config: NoiseConfig,

    pub(super) metrics: NymNodeMetrics,

    pub(super) shutdown_token: ShutdownToken,
}

impl SharedData {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        processing_config: ProcessingConfig,
        sphinx_keys: ActiveSphinxKeys,
        replay_protection_filter: ReplayProtectionBloomfilters,
        mixnet_forwarder: MixForwardingSender,
        final_hop: SharedFinalHopData,
        noise_config: NoiseConfig,
        metrics: NymNodeMetrics,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SharedData {
            processing_config,
            sphinx_keys,
            replay_protection_filter,
            mixnet_forwarder,
            final_hop,
            noise_config,
            metrics,
            shutdown_token,
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

    pub(super) fn forward_mix_packet(&self, packet: MixPacket, delay_until: Option<Instant>) {
        let has_delay = delay_until.is_some();
        if self
            .mixnet_forwarder
            .forward_packet(PacketToForward::new(packet, delay_until.map(Into::into)))
            .is_err()
            && !self.shutdown_token.is_cancelled()
        {
            error!(
                event = "forwarder.channel_send_failed",
                has_delay,
                "failed to forward sphinx packet on the channel while the process is not going through the shutdown!"
            );
            self.shutdown_token.cancel();
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
