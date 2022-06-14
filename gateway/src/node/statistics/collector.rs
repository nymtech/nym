// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use sqlx::types::chrono::{DateTime, Utc};
use std::time::Duration;

use statistics_common::{
    collector::StatisticsCollector, error::StatsError, StatsData, StatsGatewayData, StatsMessage,
};

use crate::node::client_handling::active_clients::ActiveClientsStore;

#[derive(Clone)]
pub(crate) struct GatewayStatisticsCollector {
    active_clients_store: ActiveClientsStore,
}

impl GatewayStatisticsCollector {
    pub fn new(active_clients_store: ActiveClientsStore) -> Self {
        GatewayStatisticsCollector {
            active_clients_store,
        }
    }
}

#[async_trait]
impl StatisticsCollector for GatewayStatisticsCollector {
    async fn create_stats_message(
        &self,
        interval: Duration,
        timestamp: DateTime<Utc>,
    ) -> StatsMessage {
        let inbox_count = self.active_clients_store.size() as u32;
        let stats_data = vec![StatsData::Gateway(StatsGatewayData { inbox_count })];
        StatsMessage {
            stats_data,
            interval_seconds: interval.as_secs() as u32,
            timestamp: timestamp.to_rfc3339(),
        }
    }

    fn send_stats_message(&self, _stats_message: StatsMessage) -> Result<(), StatsError> {
        Ok(())
    }

    async fn reset_stats(&mut self) {}
}
