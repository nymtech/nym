// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::config::LpConfig;
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::key_rotation::active_keys::SphinxKeyGuard;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use nym_node_metrics::NymNodeMetrics;
use nym_node_metrics::mixnet::PacketKind;
use nym_sphinx_framing::processing::{
    MixPacketVersion, MixProcessingResult, PacketProcessingError,
};
use nym_sphinx_params::SphinxKeyRotation;
use nym_task::ShutdownToken;
use std::net::IpAddr;
use std::time::Duration;
use tracing::Span;
use tracing::warn;

#[derive(Clone, Copy)]
pub(crate) struct ProcessingConfig {
    pub(crate) maximum_packet_delay: Duration,
    // /// how long the task is willing to skip mutex acquisition before it will block the thread
    // /// until it actually obtains it
    // pub(crate) maximum_replay_detection_deferral: Duration,

    // /// how many packets the task is willing to queue before it will block the thread
    // /// until it obtains the mutex
    // pub(crate) maximum_replay_detection_pending_packets: usize,
}

impl ProcessingConfig {
    pub(crate) fn new(config: &Config) -> Self {
        ProcessingConfig {
            maximum_packet_delay: config.mixnet.debug.maximum_forward_packet_delay,
            // maximum_replay_detection_deferral: config
            //     .mixnet
            //     .replay_protection
            //     .debug
            //     .maximum_replay_detection_deferral,
            // maximum_replay_detection_pending_packets: config
            //     .mixnet
            //     .replay_protection
            //     .debug
            //     .maximum_replay_detection_pending_packets,
        }
    }
}

/// Shared state for LP data connections
// explicitly do NOT derive clone as we want the childs to use CHILD shutdown tokens
pub(crate) struct SharedLpDataState {
    /// LP configuration (for timestamp validation, etc.)
    pub lp_config: LpConfig,

    pub processing_config: ProcessingConfig,

    pub sphinx_keys: ActiveSphinxKeys,

    pub replay_protection_filter: ReplayProtectionBloomfilters,

    /// Metrics collection
    pub metrics: NymNodeMetrics,

    pub shutdown_token: ShutdownToken,
}

fn convert_to_metrics_version(processed: MixPacketVersion) -> PacketKind {
    match processed {
        MixPacketVersion::Outfox => PacketKind::Outfox,
        MixPacketVersion::Sphinx(sphinx_version) => PacketKind::Sphinx(sphinx_version.value()),
    }
}

impl SharedLpDataState {
    pub(crate) fn new(
        config: &Config,
        sphinx_keys: ActiveSphinxKeys,
        replay_protection_filter: ReplayProtectionBloomfilters,
        metrics: NymNodeMetrics,
        shutdown_token: ShutdownToken,
    ) -> Self {
        SharedLpDataState {
            processing_config: ProcessingConfig::new(config),
            lp_config: config.lp,
            sphinx_keys,
            replay_protection_filter,
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

    pub(super) fn dropped_forward_packet(&self, source: IpAddr) {
        self.metrics.mixnet.ingress_dropped_forward_packet(source)
    }

    pub(super) fn dropped_final_hop_packet(&self, source: IpAddr) {
        self.metrics.mixnet.ingress_dropped_final_hop_packet(source)
    }

    // pub(super) fn update_metrics(
    //     &self,
    //     processing_result: &Result<MixProcessingResult, PacketProcessingError>,
    //     source: IpAddr,
    // ) {
    //     // let Ok(processing_result) = processing_result else {
    //     //     self.metrics.mixnet.ingress_malformed_packet(source);
    //     //     return;
    //     // };

    //     // let packet_version = convert_to_metrics_version(processing_result.packet_version);

    //     // match processing_result.processing_data {
    //     //     MixProcessingResultData::ForwardHop { delay, .. } => {
    //     //         self.metrics
    //     //             .mixnet
    //     //             .ingress_received_forward_packet(source, packet_version);

    //     //         // check if the delay wasn't excessive
    //     //         if let Some(delay) = delay
    //     //             && delay.to_duration() > self.processing_config.maximum_packet_delay
    //     //         {
    //     //             self.metrics.mixnet.ingress_excessive_delay_packet()
    //     //         }
    //     //     }
    //     //     MixProcessingResultData::FinalHop { .. } => {
    //     //         self.metrics
    //     //             .mixnet
    //     //             .ingress_received_final_hop_packet(source, packet_version);
    //     //     }
    //     // }
    //     todo!()
    // }
}
