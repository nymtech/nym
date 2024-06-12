// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use sqlx::types::chrono::{DateTime, Utc};
use std::time::Duration;
use url::Url;

use nym_statistics_common::{
    api::build_and_send_statistics_request, collector::StatisticsCollector, error::StatsError,
    StatsData, StatsGatewayData, StatsMessage,
};

use crate::node::client_handling::active_clients::ActiveClientsStore;

pub(crate) struct GatewayStatisticsCollector {
    gateway_id: String,
    active_clients_store: ActiveClientsStore,
    statistics_service_url: Url,
}

impl GatewayStatisticsCollector {
    pub fn new(
        gateway_id: String,
        active_clients_store: ActiveClientsStore,
        statistics_service_url: Url,
    ) -> Self {
        GatewayStatisticsCollector {
            gateway_id,
            active_clients_store,
            statistics_service_url,
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
        let stats_data = vec![StatsData::Gateway(StatsGatewayData::new(
            self.gateway_id.clone(),
            inbox_count,
        ))];
        StatsMessage {
            stats_data,
            interval_seconds: interval.as_secs() as u32,
            timestamp: timestamp.to_rfc3339(),
        }
    }

    async fn send_stats_message(&mut self, stats_message: StatsMessage) -> Result<(), StatsError> {
        build_and_send_statistics_request(stats_message, self.statistics_service_url.to_string())
            .await
    }

    async fn reset_stats(&mut self) {}
}
