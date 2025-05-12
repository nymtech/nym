// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymNodeError;
use crate::node::replay_protection::bloomfilter::{ReplayProtectionBloomfilters, RotationFilter};
use crate::node::replay_protection::items_in_bloomfilter;
use human_repr::HumanCount;
use nym_node_metrics::NymNodeMetrics;
use std::cmp::max;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::info;

#[derive(Clone)]
pub(crate) struct ReplayProtectionBloomfiltersManager {
    target_fp_p: f64,
    minimum_bloomfilter_packets_per_second: usize,
    bloomfilter_size_multiplier: f64,

    metrics: NymNodeMetrics,
    filters: ReplayProtectionBloomfilters,
}

impl ReplayProtectionBloomfiltersManager {
    pub(crate) fn new_disabled(metrics: NymNodeMetrics) -> Self {
        // the exact config values are irrelevant as the filters will never be recreated
        ReplayProtectionBloomfiltersManager {
            target_fp_p: 0.001,
            minimum_bloomfilter_packets_per_second: 1,
            bloomfilter_size_multiplier: 1.0,
            metrics,
            filters: ReplayProtectionBloomfilters::new_disabled(),
        }
    }

    pub(crate) fn new(
        config: &Config,
        primary: RotationFilter,
        secondary: Option<RotationFilter>,
        metrics: NymNodeMetrics,
    ) -> Self {
        ReplayProtectionBloomfiltersManager {
            target_fp_p: config.mixnet.replay_protection.debug.false_positive_rate,
            minimum_bloomfilter_packets_per_second: config
                .mixnet
                .replay_protection
                .debug
                .bloomfilter_minimum_packets_per_second_size,
            bloomfilter_size_multiplier: config
                .mixnet
                .replay_protection
                .debug
                .bloomfilter_size_multiplier,
            metrics,
            filters: ReplayProtectionBloomfilters::new(primary, secondary),
        }
    }

    pub(crate) fn bloomfilters(&self) -> ReplayProtectionBloomfilters {
        self.filters.clone()
    }

    pub(crate) fn primary_bytes_and_id(&self) -> Result<(Vec<u8>, u32), NymNodeError> {
        self.filters.primary_bytes_and_id()
    }

    pub(crate) fn secondary_bytes_and_id(&self) -> Result<Option<(Vec<u8>, u32)>, NymNodeError> {
        self.filters.secondary_bytes_and_id()
    }

    pub(crate) fn purge_secondary(&self) -> Result<(), NymNodeError> {
        self.filters.purge_secondary()
    }

    pub(crate) fn promote_pre_announced(&self) -> Result<(), NymNodeError> {
        self.filters.promote_pre_announced()
    }

    // TODO: actually do add some metrics
    pub(crate) fn allocate_pre_announced(
        &self,
        rotation_id: u32,
        rotation_lifetime: Duration,
    ) -> Result<(), NymNodeError> {
        // 1. estimated the number of items in the filter based on the extrapolated items received
        // by the primary filter
        let received = self.metrics.mixnet.ingress.forward_hop_packets_received()
            + self.metrics.mixnet.ingress.final_hop_packets_received();

        let primary = self.filters.primary_metadata()?;
        let time_delta = OffsetDateTime::now_utc() - primary.creation_time;
        let received_since_creation = received - primary.packets_received_at_creation;
        let received_per_second =
            (received_since_creation as f64 / time_delta.as_seconds_f64()).round() as usize;

        let bf_received = max(
            received_per_second,
            self.minimum_bloomfilter_packets_per_second,
        );
        let items_in_new_filter = items_in_bloomfilter(rotation_lifetime, bf_received);
        let adjusted =
            (items_in_new_filter as f64 * self.bloomfilter_size_multiplier).round() as usize;

        info!(
            "allocating new bloom filter. new expected number of packets: {} that preserve fp rate of {}",
            adjusted.human_count_bare(),
            self.target_fp_p
        );

        // 2. allocate the filter
        self.filters
            .allocate_pre_announced(adjusted, self.target_fp_p, received, rotation_id)
    }
}
