// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_http_api::state::metrics::SharedSessionStats;
use nym_sphinx::DestinationAddressBytes;
use nym_task::TaskClient;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use time::{Date, OffsetDateTime};
use tracing::trace;

use crate::node::client_handling::active_clients::ActiveClientsStore;

const STATISTICS_GATHERING_TIMER_INTERVAL: Duration = Duration::from_secs(60); //ticks for measurements
const STATISTICS_UPDATE_TIMER_INTERVAL: Duration = Duration::from_secs(3600); //update timer, no need to check everytime

type SessionDuration = u32; //number of measurements ticks

pub(crate) struct GatewayStatisticsCollector {
    gathering_interval: Duration,
    update_interval: Duration,
    last_update_day: Date,

    //settion stats_gathering
    active_clients_store: ActiveClientsStore,
    shared_session_stats: SharedSessionStats,
    active_sessions: HashMap<DestinationAddressBytes, SessionDuration>,
    unique_users: HashSet<DestinationAddressBytes>, //might be a bloom filter if this takes too much space
    sessions_started: u32,
    finished_sessions: Vec<SessionDuration>,
}

impl GatewayStatisticsCollector {
    pub fn new(
        active_clients_store: ActiveClientsStore,
        shared_session_stats: SharedSessionStats,
    ) -> Self {
        GatewayStatisticsCollector {
            active_clients_store,
            shared_session_stats,
            gathering_interval: STATISTICS_GATHERING_TIMER_INTERVAL,
            update_interval: STATISTICS_UPDATE_TIMER_INTERVAL,
            last_update_day: OffsetDateTime::now_utc().date(),
            active_sessions: Default::default(),
            unique_users: Default::default(),
            sessions_started: 0,
            finished_sessions: Default::default(),
        }
    }

    async fn gather_stats(&mut self) {
        let current_sessions = self.active_clients_store.client_list();
        let past_sessions = self.active_sessions.keys().copied().collect::<HashSet<_>>();

        //active and new sessions
        for session in &current_sessions {
            if let Some(duration) = self.active_sessions.get_mut(session) {
                *duration += 1
            } else {
                self.active_sessions.insert(*session, 1);
                self.unique_users.insert(*session);
                self.sessions_started += 1;
            }
        }

        //handling finished sessions
        for client in past_sessions.difference(&current_sessions) {
            if let Some(session_duration) = self.active_sessions.remove(client) {
                self.finished_sessions.push(session_duration);
            }
        }
    }
    //update shared state once a day has passed, with data from the previous day
    async fn update_shared_session_stats(&mut self) {
        let mut shared_state = self.shared_session_stats.write().await;
        shared_state.update_time = self.last_update_day;
        shared_state.unique_active_users = self.unique_users.len() as u32;
        shared_state.session_started = self.sessions_started;
        shared_state.session_durations = self.finished_sessions.clone();
    }

    fn reset_stats(&mut self, reset_day: Date) {
        self.last_update_day = reset_day;
        self.unique_users = self.active_sessions.keys().copied().collect();
        self.finished_sessions = Default::default();
        self.sessions_started = 0;
    }

    pub async fn run(&mut self, mut shutdown: TaskClient) {
        let mut gathering_interval = tokio::time::interval(self.gathering_interval);
        let mut update_interval = tokio::time::interval(self.update_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    trace!("StatisticsCollector: Received shutdown");
                },
                _ = update_interval.tick() => {
                    let today = OffsetDateTime::now_utc().date();
                    if today != self.last_update_day {
                        self.update_shared_session_stats().await;
                        self.reset_stats(today);
                    }

                },
                _ = gathering_interval.tick() => {
                    self.gather_stats().await;
                }

            }
        }
    }
}
