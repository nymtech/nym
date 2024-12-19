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
        use PrometheusMetric::*;

        // # MIXNET
        // ## INGRESS
        self.prometheus_wrapper.set(
            MixnetIngressForwardPacketsReceived,
            self.metrics.mixnet.ingress.forward_hop_packets_received() as i64,
        );
        self.prometheus_wrapper.set(
            MixnetIngressFinalHopPacketsReceived,
            self.metrics.mixnet.ingress.final_hop_packets_received() as i64,
        );
        self.prometheus_wrapper.set(
            MixnetIngressMalformedPacketsReceived,
            self.metrics.mixnet.ingress.malformed_packets_received() as i64,
        );
        self.prometheus_wrapper.set(
            MixnetIngressExcessiveDelayPacketsReceived,
            self.metrics.mixnet.ingress.excessive_delay_packets() as i64,
        );
        self.prometheus_wrapper.set(
            MixnetEgressForwardPacketsDropped,
            self.metrics.mixnet.ingress.forward_hop_packets_dropped() as i64,
        );
        self.prometheus_wrapper.set(
            MixnetIngressFinalHopPacketsDropped,
            self.metrics.mixnet.ingress.final_hop_packets_dropped() as i64,
        );

        // ## EGRESS
        self.prometheus_wrapper.set(
            MixnetEgressForwardPacketsSent,
            self.metrics.mixnet.egress.forward_hop_packets_sent() as i64,
        );
        self.prometheus_wrapper.set(
            MixnetEgressAckSent,
            self.metrics.mixnet.egress.ack_packets_sent() as i64,
        );
        self.prometheus_wrapper.set(
            MixnetEgressForwardPacketsDropped,
            self.metrics.mixnet.egress.forward_hop_packets_dropped() as i64,
        );

        // # ENTRY
        self.prometheus_wrapper.set(
            EntryClientUniqueUsers,
            entry_guard.unique_users.len() as i64,
        );
        self.prometheus_wrapper.set(
            EntryClientSessionsStarted,
            entry_guard.sessions_started as i64,
        );
        self.prometheus_wrapper.set(
            EntryClientSessionsFinished,
            entry_guard.finished_sessions.len() as i64,
        );

        // # WIREGUARD
        self.prometheus_wrapper
            .set(WireguardBytesRx, self.metrics.wireguard.bytes_rx() as i64);
        self.prometheus_wrapper
            .set(WireguardBytesTx, self.metrics.wireguard.bytes_tx() as i64);
        self.prometheus_wrapper.set(
            WireguardTotalPeers,
            self.metrics.wireguard.total_peers() as i64,
        );
        self.prometheus_wrapper.set(
            WireguardActivePeers,
            self.metrics.wireguard.active_peers() as i64,
        );

        // # NETWORK
        self.prometheus_wrapper.set(
            NetworkActiveIngressMixnetConnections,
            self.metrics
                .network
                .active_ingress_mixnet_connections_count() as i64,
        );
        self.prometheus_wrapper.set(
            NetworkActiveIngressWebSocketConnections,
            self.metrics
                .network
                .active_ingress_websocket_connections_count() as i64,
        );
        self.prometheus_wrapper.set(
            NetworkActiveIngressWebSocketConnections,
            self.metrics
                .network
                .active_egress_mixnet_connections_count() as i64,
        );

        let updated = AtLastUpdate::from(&self.metrics);

        // # RATES
        if !self.at_last_update.is_initial() {
            let diff = updated.rates(&self.at_last_update);

            self.prometheus_wrapper.set_float(
                MixnetIngressForwardPacketsReceivedRate,
                diff.mixnet.ingress.forward_hop_packets_received_sec,
            );
            self.prometheus_wrapper.set_float(
                MixnetIngressFinalHopPacketsReceivedRate,
                diff.mixnet.ingress.final_hop_packets_received_sec,
            );
            self.prometheus_wrapper.set_float(
                MixnetIngressMalformedPacketsReceivedRate,
                diff.mixnet.ingress.malformed_packets_received_sec,
            );
            self.prometheus_wrapper.set_float(
                MixnetIngressExcessiveDelayPacketsReceivedRate,
                diff.mixnet.ingress.excessive_delay_packets_sec,
            );
            self.prometheus_wrapper.set_float(
                MixnetIngressForwardPacketsDroppedRate,
                diff.mixnet.ingress.forward_hop_packets_dropped_sec,
            );
            self.prometheus_wrapper.set_float(
                MixnetIngressFinalHopPacketsDroppedRate,
                diff.mixnet.ingress.final_hop_packets_dropped_sec,
            );

            // ## EGRESS
            self.prometheus_wrapper.set_float(
                MixnetEgressForwardPacketsSentRate,
                diff.mixnet.egress.forward_hop_packets_sent_sec,
            );
            self.prometheus_wrapper.set_float(
                MixnetEgressAckSentRate,
                diff.mixnet.egress.ack_packets_sent_sec,
            );
            self.prometheus_wrapper.set_float(
                MixnetEgressForwardPacketsDroppedRate,
                diff.mixnet.egress.forward_hop_packets_dropped_sec,
            );

            // # WIREGUARD
            self.prometheus_wrapper
                .set_float(WireguardBytesRxRate, diff.wireguard.bytes_rx_sec);
            self.prometheus_wrapper
                .set_float(WireguardBytesTxRate, diff.wireguard.bytes_tx_sec);
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
