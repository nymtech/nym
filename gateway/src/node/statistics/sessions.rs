// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_http_api::state::metrics::SharedSessionStats;
use nym_sphinx::DestinationAddressBytes;
use std::collections::{HashMap, HashSet};
use time::{Date, OffsetDateTime};

use nym_statistics_common::events::SessionEvent;

type SessionDuration = u64; //in miliseconds

pub(crate) struct SessionStatsHandler {
    last_update_day: Date,

    shared_session_stats: SharedSessionStats,
    active_sessions: HashMap<DestinationAddressBytes, OffsetDateTime>,
    unique_users: HashSet<DestinationAddressBytes>,
    sessions_started: u32,
    finished_sessions: Vec<SessionDuration>,
}

impl SessionStatsHandler {
    pub fn new(shared_session_stats: SharedSessionStats) -> Self {
        SessionStatsHandler {
            last_update_day: OffsetDateTime::now_utc().date(),
            shared_session_stats,
            active_sessions: Default::default(),
            unique_users: Default::default(),
            sessions_started: 0,
            finished_sessions: Default::default(),
        }
    }

    pub(crate) fn handle_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::SessionStart { start_time, client } => {
                self.handle_session_start(start_time, client);
            }
            SessionEvent::SessionStop { stop_time, client } => {
                self.handle_session_stop(stop_time, client);
            }
        }
    }
    fn handle_session_start(
        &mut self,
        start_time: OffsetDateTime,
        client: DestinationAddressBytes,
    ) {
        self.sessions_started += 1;
        self.unique_users.insert(client);
        self.active_sessions.insert(client, start_time);
    }
    fn handle_session_stop(&mut self, stop_time: OffsetDateTime, client: DestinationAddressBytes) {
        if let Some(session_start) = self.active_sessions.remove(&client) {
            let session_duration = (stop_time - session_start).whole_milliseconds();

            //this should always happen because it should always be positive and u64::max milliseconds is 500k millenia, but anyway
            if let Ok(duration_u64) = session_duration.try_into() {
                self.finished_sessions.push(duration_u64);
            }
        }
    }

    //update shared state once a day has passed, with data from the previous day
    pub(crate) async fn update_shared_state(&mut self, update_time: OffsetDateTime) {
        let update_date = update_time.date();
        if update_date != self.last_update_day {
            let mut shared_state = self.shared_session_stats.write().await;
            shared_state.update_time = self.last_update_day;
            shared_state.unique_active_users = self.unique_users.len() as u32;
            shared_state.session_started = self.sessions_started;
            shared_state.session_durations = self.finished_sessions.clone();
            drop(shared_state);
            self.reset_stats(update_date);
        }
    }

    fn reset_stats(&mut self, reset_day: Date) {
        self.last_update_day = reset_day;
        self.unique_users = self.active_sessions.keys().copied().collect();
        self.finished_sessions = Default::default();
        self.sessions_started = 0;
    }
}
