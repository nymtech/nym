// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::config::LpConfig;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::key_rotation::active_keys::SphinxKeyGuard;
use crate::node::lp::data::handler::error::LpDataHandlerError;
use crate::node::lp::data::handler::messages::MixMessage;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use crate::node::routing_filter::network_filter::NetworkRoutingFilter;
use nym_lp_data::PipelinePayload;
use nym_lp_data::fragmentation::reconstruction::MessageReconstructor;
use nym_node_metrics::NymNodeMetrics;
use nym_node_metrics::mixnet::PacketKind;
use nym_sphinx_framing::processing::PacketProcessingError;
use nym_sphinx_params::SphinxKeyRotation;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use std::time::Duration;
use std::time::Instant;
use tracing::Span;
use tracing::warn;

#[derive(Clone, Copy)]
pub(crate) struct ProcessingConfig {
    pub(crate) maximum_packet_delay: Duration,
}

impl ProcessingConfig {
    pub(crate) fn new(config: &Config) -> Self {
        ProcessingConfig {
            maximum_packet_delay: config.mixnet.debug.maximum_forward_packet_delay,
        }
    }
}

/// Shared state for LP data connections
pub(crate) struct SharedLpDataState {
    /// LP configuration (for timestamp validation, etc.)
    pub lp_config: LpConfig,

    pub processing_config: ProcessingConfig,

    pub sphinx_keys: ActiveSphinxKeys,

    pub replay_protection_filter: ReplayProtectionBloomfilters,

    pub message_reconstructor: MessageReconstructor<Instant, Duration>,

    pub routing_filter: NetworkRoutingFilter,

    /// Metrics collection
    pub metrics: NymNodeMetrics,

    pub shutdown_token: ShutdownToken,
}

fn message_kind_to_packet_kind(message_kind: &MixMessage) -> PacketKind {
    match message_kind {
        MixMessage::Sphinx { .. } => PacketKind::LpSphinx,
        MixMessage::Outfox { .. } => PacketKind::LpOutfox,
    }
}

impl SharedLpDataState {
    pub(crate) fn new(
        config: &Config,
        sphinx_keys: ActiveSphinxKeys,
        replay_protection_filter: ReplayProtectionBloomfilters,
        routing_filter: NetworkRoutingFilter,
        metrics: NymNodeMetrics,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SharedLpDataState {
            processing_config: ProcessingConfig::new(config),
            lp_config: config.lp,
            sphinx_keys,
            replay_protection_filter,
            message_reconstructor: Default::default(),
            routing_filter,
            metrics,
            shutdown_token,
        }
    }

    /// Resolve the sphinx key for the given rotation, recording the rotation
    /// label on the current tracing span.  Returns `ExpiredKey` if the requested
    /// odd/even key has already been rotated out.
    pub(crate) fn resolve_rotation_key(
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
            SphinxKeyRotation::Unknown => Ok(self.sphinx_keys.primary()),
            SphinxKeyRotation::OddRotation => self.sphinx_keys.odd().ok_or_else(|| {
                warn!(
                    event = "packet.dropped.expired_key",
                    key_rotation = "odd",
                    "dropping packet: odd key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
            SphinxKeyRotation::EvenRotation => self.sphinx_keys.even().ok_or_else(|| {
                warn!(
                    event = "packet.dropped.expired_key",
                    key_rotation = "even",
                    "dropping packet: even key rotation expired"
                );
                PacketProcessingError::ExpiredKey
            }),
        }
    }

    pub(super) fn malformed_packet(&self) {
        self.metrics.mixnet.lp_malformed_packet()
    }

    pub(super) fn message_received(&self, message_kind: &MixMessage) {
        self.metrics
            .mixnet
            .lp_message_received(message_kind_to_packet_kind(message_kind))
    }

    pub(super) fn packet_forwarded(&self, dst: SocketAddr) {
        self.metrics.mixnet.lp_packet_forwarded(dst)
    }

    pub(super) fn packet_received(&self, src: SocketAddr) {
        self.metrics.mixnet.lp_packet_received(src)
    }

    pub(super) fn egress_overloaded_packet_dropped(&self) {
        self.metrics
            .mixnet
            .lp_egress_overloaded_packets_dropped_packets()
    }
    pub(super) fn pipeline_overloaded_packet_dropped(&self) {
        self.metrics.mixnet.lp_pipeline_overloaded_dropped_packets()
    }

    pub(super) fn worker_pool_overloaded_packet_dropped(&self) {
        self.metrics
            .mixnet
            .lp_worker_pool_overloaded_dropped_packets()
    }

    pub(super) fn excessive_delay_packet(&self) {
        self.metrics.mixnet.lp_excessive_delay_packet()
    }

    pub(super) fn routing_filter_dropped(&self, dst: SocketAddr) {
        self.metrics.mixnet.lp_routing_filter_dropped(dst)
    }

    pub(super) fn update_processing_metrics(
        &self,
        processing_result: &Result<
            PipelinePayload<Instant, MixMessage, SocketAddr>,
            LpDataHandlerError,
        >,
    ) {
        match processing_result {
            Ok(packet) => {
                self.metrics
                    .mixnet
                    .lp_processed_message(message_kind_to_packet_kind(&packet.options));
            }
            Err(LpDataHandlerError::PacketProcessingError(PacketProcessingError::PacketReplay)) => {
                self.metrics.mixnet.lp_processing_replayed_packet();
            }
            Err(LpDataHandlerError::FinalHop) => {
                self.metrics.mixnet.lp_processing_dropped_final_hop_packet();
            }
            Err(_) => {
                self.metrics.mixnet.lp_processing_misc_error();
            }
        }
    }
}
