// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::global_prometheus_updater::at_last_update::AtLastUpdate;
use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_metrics::set_metric;
use nym_node_metrics::mixnet::IngressMixingStats;
use nym_node_metrics::NymNodeMetrics;
use time::OffsetDateTime;

mod at_last_update;

// it can be anything, we just need a unique type_id to register our handler
pub struct GlobalPrometheusData;

pub struct PrometheusGlobalNodeMetricsRegistryUpdater {
    metrics: NymNodeMetrics,
    at_last_update: AtLastUpdate,
}

impl PrometheusGlobalNodeMetricsRegistryUpdater {
    pub(crate) fn new(metrics: NymNodeMetrics) -> Self {
        Self {
            metrics,
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
        set_metric!(
            "mixnet_ingress_forward_hop_packets_received",
            self.metrics.mixnet.ingress.forward_hop_packets_received()
        );
        set_metric!(
            "mixnet_ingress_final_hop_packets_received",
            self.metrics.mixnet.ingress.final_hop_packets_received()
        );
        set_metric!(
            "mixnet_ingress_malformed_packets_received",
            self.metrics.mixnet.ingress.malformed_packets_received()
        );
        set_metric!(
            "mixnet_ingress_excessive_delay_packets",
            self.metrics.mixnet.ingress.excessive_delay_packets()
        );
        set_metric!(
            "mixnet_ingress_forward_hop_packets_dropped",
            self.metrics.mixnet.ingress.forward_hop_packets_dropped()
        );
        set_metric!(
            "mixnet_ingress_final_hop_packets_dropped",
            self.metrics.mixnet.ingress.final_hop_packets_dropped()
        );
        // set_metric!("mixnet_ingress_senders", )

        // ## EGRESS
        set_metric!(
            "mixnet_egress_forward_hop_packets_sent",
            self.metrics.mixnet.egress.forward_hop_packets_sent()
        );
        set_metric!(
            "mixnet_egress_ack_packets_sent",
            self.metrics.mixnet.egress.ack_packets_sent()
        );
        set_metric!(
            "mixnet_egress_forward_hop_packets_dropped",
            self.metrics.mixnet.egress.forward_hop_packets_dropped()
        );
        // set_metric!("mixnet_egress_forward_recipients", )

        // # ENTRY
        set_metric!(
            "entry_client_sessions_unique_users",
            entry_guard.unique_users.len()
        );
        set_metric!(
            "entry_client_sessions_sessions_started",
            entry_guard.sessions_started
        );
        set_metric!(
            "entry_client_sessions_finished_sessions",
            entry_guard.finished_sessions.len()
        );
        // histograms for finished sessions duration/typ

        // # WIREGUARD
        set_metric!("wireguard_bytes_rx", self.metrics.wireguard.bytes_rx());
        set_metric!("wireguard_bytes_tx", self.metrics.wireguard.bytes_tx());
        set_metric!(
            "wireguard_bytes_total_peers",
            self.metrics.wireguard.total_peers()
        );
        set_metric!(
            "wireguard_bytes_active_peers",
            self.metrics.wireguard.active_peers()
        );

        // # NETWORK
        set_metric!(
            "network_active_ingress_mixnet_connections",
            self.metrics
                .network
                .active_ingress_mixnet_connections_count()
        );

        let updated = AtLastUpdate::from(&self.metrics);

        // # RATES

        if !self.at_last_update.is_initial() {
            let diff = updated.rates(&self.at_last_update);
            set_metric!(
                "mixnet_ingress_forward_hop_packets_received_rate",
                diff.mixnet.ingress.forward_hop_packets_received_sec
            );
            set_metric!(
                "mixnet_ingress_final_hop_packets_received_rate",
                diff.mixnet.ingress.final_hop_packets_received_sec
            );
            set_metric!(
                "mixnet_ingress_malformed_packets_received_rate",
                diff.mixnet.ingress.malformed_packets_received_sec
            );
            set_metric!(
                "mixnet_ingress_excessive_delay_packets_rate",
                diff.mixnet.ingress.excessive_delay_packets_sec
            );
            set_metric!(
                "mixnet_ingress_forward_hop_packets_dropped_rate",
                diff.mixnet.ingress.forward_hop_packets_dropped_sec
            );
            set_metric!(
                "mixnet_ingress_final_hop_packets_dropped_rate",
                diff.mixnet.ingress.final_hop_packets_dropped_sec
            );

            // ## EGRESS
            set_metric!(
                "mixnet_egress_forward_hop_packets_sent_rate",
                diff.mixnet.egress.forward_hop_packets_sent_sec
            );
            set_metric!(
                "mixnet_egress_ack_packets_sent_rate",
                diff.mixnet.egress.ack_packets_sent_sec
            );
            set_metric!(
                "mixnet_egress_forward_hop_packets_dropped_rate",
                diff.mixnet.egress.forward_hop_packets_dropped_sec
            );

            // # WIREGUARD
            set_metric!("wireguard_bytes_rx_rate", diff.wireguard.bytes_rx_sec);
            set_metric!("wireguard_bytes_tx_rate", diff.wireguard.bytes_tx_sec);
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
