// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_node_metrics::NymNodeMetrics;
use time::OffsetDateTime;

// it can be anything, we just need a unique type_id to register our handler
pub struct LegacyMixingData;

pub struct LegacyMixingStatsUpdater {
    received_total_at_last_update: usize,
    dropped_total_at_last_update: usize,
    sent_total_at_last_update: usize,

    metrics: NymNodeMetrics,
}

impl LegacyMixingStatsUpdater {
    pub(crate) fn new(metrics: NymNodeMetrics) -> Self {
        LegacyMixingStatsUpdater {
            received_total_at_last_update: 0,
            dropped_total_at_last_update: 0,
            sent_total_at_last_update: 0,
            metrics,
        }
    }
}

#[async_trait]
impl OnStartMetricsHandler for LegacyMixingStatsUpdater {}

#[async_trait]
impl OnUpdateMetricsHandler for LegacyMixingStatsUpdater {
    async fn on_update(&mut self) {
        let total_received = self.metrics.mixnet.ingress.forward_hop_packets_received();
        let total_dropped = self.metrics.mixnet.egress.forward_hop_packets_dropped();
        let total_sent = self.metrics.mixnet.egress.forward_hop_packets_sent();

        let received_since_update =
            total_received.saturating_sub(self.received_total_at_last_update);
        let dropped_since_update = total_dropped.saturating_sub(self.dropped_total_at_last_update);
        let sent_since_update = total_sent.saturating_sub(self.sent_total_at_last_update);

        self.received_total_at_last_update = total_received;
        self.sent_total_at_last_update = total_sent;
        self.dropped_total_at_last_update = total_dropped;

        self.metrics.mixnet.update_legacy_stats(
            received_since_update,
            sent_since_update,
            dropped_since_update,
            OffsetDateTime::now_utc().unix_timestamp(),
        );
    }
}

#[async_trait]
impl MetricsHandler for LegacyMixingStatsUpdater {
    type Events = LegacyMixingData;

    // SAFETY: `LegacyMixingStatsUpdater` doesn't have any associated events
    #[allow(clippy::panic)]
    async fn handle_event(&mut self, _event: Self::Events) {
        panic!("this should have never been called! MetricsHandler has been incorrectly called on LegacyMixingStatsUpdater")
    }
}
