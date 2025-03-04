// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_gateway::node::ActiveClientsStore;
use nym_mixnet_client::client::ActiveConnections;
use nym_node_metrics::NymNodeMetrics;

// it can be anything, we just need a unique type_id to register our handler
pub struct PendingEgressPackets;

pub struct PendingEgressPacketsUpdater {
    metrics: NymNodeMetrics,
    active_websocket_clients: ActiveClientsStore,
    active_mixnet_connections: ActiveConnections,
}

impl PendingEgressPacketsUpdater {
    pub(crate) fn new(
        metrics: NymNodeMetrics,
        active_clients: ActiveClientsStore,
        active_mixnet_connections: ActiveConnections,
    ) -> Self {
        PendingEgressPacketsUpdater {
            metrics,
            active_websocket_clients: active_clients,
            active_mixnet_connections,
        }
    }
}

#[async_trait]
impl OnStartMetricsHandler for PendingEgressPacketsUpdater {}

#[async_trait]
impl OnUpdateMetricsHandler for PendingEgressPacketsUpdater {
    async fn on_update(&mut self) {
        let pending_final = self.active_websocket_clients.pending_packets();
        self.metrics
            .process
            .update_final_hop_packets_pending_delivery(pending_final);

        let pending_forward = self.active_mixnet_connections.pending_packets();
        self.metrics
            .process
            .update_forward_hop_packets_pending_delivery(pending_forward)
    }
}

#[async_trait]
impl MetricsHandler for PendingEgressPacketsUpdater {
    type Events = PendingEgressPackets;

    async fn handle_event(&mut self, _event: Self::Events) {
        panic!("this should have never been called! MetricsHandler has been incorrectly called on PendingEgressPacketsUpdater")
    }
}
