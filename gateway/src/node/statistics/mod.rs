// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::{channel::mpsc, StreamExt};
use nym_node_http_api::state::metrics::SharedSessionStats;
use nym_statistics_common::events::{StatsEvent, StatsEventReceiver, StatsEventSender};
use nym_task::TaskClient;
use sessions::SessionStatsHandler;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::trace;

pub mod sessions;

const STATISTICS_UPDATE_TIMER_INTERVAL: Duration = Duration::from_secs(3600); //update timer, no need to check everytime

pub(crate) struct GatewayStatisticsCollector {
    stats_event_rx: StatsEventReceiver,
    session_stats: SessionStatsHandler,
    //here goes additionnal stats handler
}

impl GatewayStatisticsCollector {
    pub fn new(
        shared_session_stats: SharedSessionStats,
    ) -> (GatewayStatisticsCollector, StatsEventSender) {
        let (stats_event_tx, stats_event_rx) = mpsc::unbounded();
        let collector = GatewayStatisticsCollector {
            stats_event_rx,
            session_stats: SessionStatsHandler::new(shared_session_stats),
        };
        (collector, stats_event_tx)
    }

    async fn update_shared_state(&mut self, update_time: OffsetDateTime) {
        self.session_stats.update_shared_state(update_time).await;
        //here goes additionnal stats handler update
    }

    pub async fn run(&mut self, mut shutdown: TaskClient) {
        let mut update_interval = tokio::time::interval(STATISTICS_UPDATE_TIMER_INTERVAL);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    trace!("StatisticsCollector: Received shutdown");
                },
                _ = update_interval.tick() => {
                    let now = OffsetDateTime::now_utc();
                        self.update_shared_state(now).await;
                },

                Some(stat_event) = self.stats_event_rx.next() => {
                    //dispatching event to proper handler
                    match stat_event {
                        StatsEvent::SessionStatsEvent(event) => self.session_stats.handle_event(event),
                    }
                },

            }
        }
    }
}
