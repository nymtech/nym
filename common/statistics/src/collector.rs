// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::error;
use sqlx::types::chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::time;

use crate::error::StatsError;
use crate::StatsMessage;

const STATISTICS_TIMER_INTERVAL: Duration = Duration::from_secs(60);

#[async_trait]
pub trait StatisticsCollector {
    async fn create_stats_message(
        &self,
        interval: Duration,
        timestamp: DateTime<Utc>,
    ) -> StatsMessage;
    async fn send_stats_message(&mut self, stats_message: StatsMessage) -> Result<(), StatsError>;
    async fn reset_stats(&mut self);
}

pub struct StatisticsSender<T: StatisticsCollector> {
    collector: T,
    interval: Duration,
    timestamp: DateTime<Utc>,
}

impl<T: StatisticsCollector> StatisticsSender<T> {
    pub fn new(collector: T) -> Self {
        StatisticsSender {
            collector,
            interval: STATISTICS_TIMER_INTERVAL,
            timestamp: Utc::now(),
        }
    }

    pub async fn run(&mut self) {
        let mut interval = time::interval(self.interval);
        loop {
            interval.tick().await;

            let stats_message = self
                .collector
                .create_stats_message(self.interval, self.timestamp)
                .await;
            if let Err(e) = self.collector.send_stats_message(stats_message).await {
                error!("Statistics not sent: {}", e);
            }
            self.collector.reset_stats().await;
            self.timestamp = Utc::now();
        }
    }
}
