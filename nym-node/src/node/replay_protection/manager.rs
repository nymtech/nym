// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use crate::node::replay_protection::items_in_bloomfilter;
use human_repr::HumanCount;
use nym_node_metrics::NymNodeMetrics;
use std::cmp::max;
use std::time::Duration;
use tracing::info;

pub(crate) struct ReplayProtectionBloomfiltersManager {
    target_fp_p: f64,
    minimum_bloomfilter_packets_per_second: usize,
    bloomfilter_size_multiplier: f64,

    metrics: NymNodeMetrics,
    filters: ReplayProtectionBloomfilters,
}

impl ReplayProtectionBloomfiltersManager {
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
        let time_delta = primary.creation_time.elapsed();
        let received_since_creation = received - primary.packets_received_at_creation;
        let received_per_second =
            (received_since_creation as f64 / time_delta.as_secs_f64()).round() as usize;

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

        self.filters
            .allocate_pre_announced(adjusted, self.target_fp_p, received, rotation_id)
    }
}
