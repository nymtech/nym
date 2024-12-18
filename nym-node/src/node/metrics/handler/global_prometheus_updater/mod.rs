// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::global_prometheus_updater::at_last_update::AtLastUpdate;
use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_node_metrics::prometheus_wrapper::{
    NymNodePrometheusMetrics, PrometheusMetric, PROMETHEUS_METRICS,
};
use nym_node_metrics::NymNodeMetrics;

mod at_last_update;

const CLIENT_SESSION_DURATION_BUCKETS: &[f64] = &[
    // sub 3s (implicitly)
    3.,      // 3s - 15s
    15.,     // 15s - 70s
    70.,     // 70s - 2min
    120.,    // 2 min - 5 min
    300.,    // 5min - 15min
    900.,    // 15min - 1h
    3600.,   // 1h - 12h
    43200.,  // 12h - 23.5h
    88200.,  // 23.5h - 24.5h
    86400.,  // 24.5h - 72h
    259200., // 72h+ (implicitly)
];

// it can be anything, we just need a unique type_id to register our handler
pub struct GlobalPrometheusData;

pub struct PrometheusGlobalNodeMetricsRegistryUpdater {
    metrics: NymNodeMetrics,
    prometheus_wrapper: &'static NymNodePrometheusMetrics,
    at_last_update: AtLastUpdate,
}

impl PrometheusGlobalNodeMetricsRegistryUpdater {
    pub(crate) fn new(metrics: NymNodeMetrics) -> Self {
        Self {
            metrics,
            prometheus_wrapper: &PROMETHEUS_METRICS,
            at_last_update: Default::default(),
        }
    }
}

#[async_trait]
impl OnStartMetricsHandler for PrometheusGlobalNodeMetricsRegistryUpdater {}

#[async_trait]
impl OnUpdateMetricsHandler for PrometheusGlobalNodeMetricsRegistryUpdater {
    async fn on_update(&mut self) {
        let entry_guard = self.metrics.entry.client_sessions().await;

        // # MIXNET
        // ## INGRESS
        PrometheusMetric::MixnetIngressForwardPacketsReceived
            .set(self.metrics.mixnet.ingress.forward_hop_packets_received() as i64);
        PrometheusMetric::MixnetIngressFinalHopPacketsReceived
            .set(self.metrics.mixnet.ingress.final_hop_packets_received() as i64);
        PrometheusMetric::MixnetIngressMalformedPacketsReceived
            .set(self.metrics.mixnet.ingress.malformed_packets_received() as i64);
        PrometheusMetric::MixnetIngressExcessiveDelayPacketsReceived
            .set(self.metrics.mixnet.ingress.excessive_delay_packets() as i64);
        PrometheusMetric::MixnetEgressForwardPacketsDropped
            .set(self.metrics.mixnet.ingress.forward_hop_packets_dropped() as i64);
        PrometheusMetric::MixnetIngressFinalHopPacketsDropped
            .set(self.metrics.mixnet.ingress.final_hop_packets_dropped() as i64);

        // ## EGRESS
        PrometheusMetric::MixnetEgressForwardPacketsSent
            .set(self.metrics.mixnet.egress.forward_hop_packets_sent() as i64);
        PrometheusMetric::MixnetEgressAckSent
            .set(self.metrics.mixnet.egress.ack_packets_sent() as i64);
        PrometheusMetric::MixnetEgressForwardPacketsDropped
            .set(self.metrics.mixnet.egress.forward_hop_packets_dropped() as i64);

        // # ENTRY
        PrometheusMetric::EntryClientUniqueUsers.set(entry_guard.unique_users.len() as i64);
        PrometheusMetric::EntryClientSessionsStarted.set(entry_guard.sessions_started as i64);
        PrometheusMetric::EntryClientSessionsFinished
            .set(entry_guard.finished_sessions.len() as i64);

        for session in &entry_guard.finished_sessions {
            let typ = session.typ.to_string();
            let duration = session.duration.as_secs_f64();
            PrometheusMetric::EntryClientSessionsDurations { typ }.observe_histogram(duration);
        }

        // # WIREGUARD
        PrometheusMetric::WireguardBytesRx.set(self.metrics.wireguard.bytes_rx() as i64);
        PrometheusMetric::WireguardBytesTx.set(self.metrics.wireguard.bytes_tx() as i64);
        PrometheusMetric::WireguardTotalPeers.set(self.metrics.wireguard.total_peers() as i64);
        PrometheusMetric::WireguardActivePeers.set(self.metrics.wireguard.active_peers() as i64);

        // # NETWORK
        PrometheusMetric::NetworkActiveIngressMixnetConnections.set(
            self.metrics
                .network
                .active_ingress_mixnet_connections_count() as i64,
        );

        let updated = AtLastUpdate::from(&self.metrics);

        // # RATES
        if !self.at_last_update.is_initial() {
            let diff = updated.rates(&self.at_last_update);

            PrometheusMetric::MixnetIngressForwardPacketsReceivedRate
                .set_float(diff.mixnet.ingress.forward_hop_packets_received_sec);
            PrometheusMetric::MixnetIngressFinalHopPacketsReceivedRate
                .set_float(diff.mixnet.ingress.final_hop_packets_received_sec);
            PrometheusMetric::MixnetIngressMalformedPacketsReceivedRate
                .set_float(diff.mixnet.ingress.malformed_packets_received_sec);
            PrometheusMetric::MixnetIngressExcessiveDelayPacketsReceivedRate
                .set_float(diff.mixnet.ingress.excessive_delay_packets_sec);
            PrometheusMetric::MixnetIngressForwardPacketsDroppedRate
                .set_float(diff.mixnet.ingress.forward_hop_packets_dropped_sec);
            PrometheusMetric::MixnetIngressFinalHopPacketsDroppedRate
                .set_float(diff.mixnet.ingress.final_hop_packets_dropped_sec);

            // ## EGRESS
            PrometheusMetric::MixnetEgressForwardPacketsSentRate
                .set_float(diff.mixnet.egress.forward_hop_packets_sent_sec);
            PrometheusMetric::MixnetEgressAckSentRate
                .set_float(diff.mixnet.egress.ack_packets_sent_sec);
            PrometheusMetric::MixnetEgressForwardPacketsDroppedRate
                .set_float(diff.mixnet.egress.forward_hop_packets_dropped_sec);

            // # WIREGUARD
            PrometheusMetric::WireguardBytesRxRate.set_float(diff.wireguard.bytes_rx_sec);
            PrometheusMetric::WireguardBytesTxRate.set_float(diff.wireguard.bytes_tx_sec);
        }
        self.at_last_update = updated;
    }
}

#[async_trait]
impl MetricsHandler for PrometheusGlobalNodeMetricsRegistryUpdater {
    type Events = GlobalPrometheusData;

    async fn handle_event(&mut self, _event: Self::Events) {
        panic!("this should have never been called! MetricsHandler has been incorrectly called on PrometheusNodeMetricsRegistryUpdater")
    }
}
