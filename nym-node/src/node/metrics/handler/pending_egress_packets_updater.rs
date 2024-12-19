// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::metrics::handler::{
    MetricsHandler, OnStartMetricsHandler, OnUpdateMetricsHandler,
};
use async_trait::async_trait;
use nym_gateway::node::ActiveClientsStore;
use nym_node_metrics::NymNodeMetrics;

// it can be anything, we just need a unique type_id to register our handler
pub struct PendingEgressPackets;

pub struct PendingEgressPacketsUpdater {
    metrics: NymNodeMetrics,
    active_clients: ActiveClientsStore,
}

impl PendingEgressPacketsUpdater {
    pub(crate) fn new(metrics: NymNodeMetrics, active_clients: ActiveClientsStore) -> Self {
        PendingEgressPacketsUpdater {
            metrics,
            active_clients,
        }
    }
}

#[async_trait]
impl OnStartMetricsHandler for PendingEgressPacketsUpdater {}

#[async_trait]
impl OnUpdateMetricsHandler for PendingEgressPacketsUpdater {
    async fn on_update(&mut self) {
        let pending_packets = self.active_clients.pending_packets();
        self.metrics
            .process
            .update_final_hop_packets_pending_delivery(pending_packets)
    }
}

#[async_trait]
impl MetricsHandler for PendingEgressPacketsUpdater {
    type Events = PendingEgressPackets;

    async fn handle_event(&mut self, _event: Self::Events) {
        panic!("this should have never been called! MetricsHandler has been incorrectly called on PendingEgressPacketsUpdater")
    }
}
