// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials_interface::TicketType;
use nym_node_http_api::state::metrics::SharedSessionStats;
use nym_sphinx::DestinationAddressBytes;
use std::collections::{HashMap, HashSet};
use time::{Date, Duration, OffsetDateTime};

use nym_statistics_common::events::SessionEvent;

#[derive(PartialEq)]
enum SessionType {
    Vpn,
    Mixnet,
    Unknown,
}

impl SessionType {
    fn to_string(&self) -> &str {
        match self {
            Self::Vpn => "vpn",
            Self::Mixnet => "mixnet",
            Self::Unknown => "unknown",
        }
    }
}

impl From<TicketType> for SessionType {
    fn from(value: TicketType) -> Self {
        match value {
            TicketType::V1MixnetEntry => Self::Mixnet,
            TicketType::V1MixnetExit => Self::Mixnet,
            TicketType::V1WireguardEntry => Self::Vpn,
            TicketType::V1WireguardExit => Self::Vpn,
        }
    }
}

struct FinishedSession {
    duration: Duration,
    typ: SessionType,
}

impl FinishedSession {
    fn serialize(&self) -> (u64, String) {
        (
            self.duration.whole_milliseconds() as u64, //we are sure that it fits in a u64, see `fn end_at`
            self.typ.to_string().into(),
        )
    }
}

struct ActiveSession {
    start: OffsetDateTime,
    typ: SessionType,
}

impl ActiveSession {
    fn new(start_time: OffsetDateTime) -> Self {
        ActiveSession {
            start: start_time,
            typ: SessionType::Unknown,
        }
    }

    fn set_type(&mut self, ticket_type: TicketType) {
        self.typ = ticket_type.into();
    }

    fn end_at(self, stop_time: OffsetDateTime) -> Option<FinishedSession> {
        let session_duration = stop_time - self.start;
        //ensure duration is positive to fit in a u64
        //u64::max milliseconds is 500k millenia so no overflow issue
        if session_duration > Duration::ZERO {
            Some(FinishedSession {
                duration: session_duration,
                typ: self.typ,
            })
        } else {
            None
        }
    }
}

pub(crate) struct SessionStatsHandler {
    last_update_day: Date,

    shared_session_stats: SharedSessionStats,
    active_sessions: HashMap<DestinationAddressBytes, ActiveSession>,
    unique_users: HashSet<DestinationAddressBytes>,
    sessions_started: u32,
    finished_sessions: Vec<FinishedSession>,
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
            SessionEvent::EcashTicket {
                ticket_type,
                client,
            } => self.handle_ecash_ticket(ticket_type, client),
        }
    }
    fn handle_session_start(
        &mut self,
        start_time: OffsetDateTime,
        client: DestinationAddressBytes,
    ) {
        self.sessions_started += 1;
        self.unique_users.insert(client);
        self.active_sessions
            .insert(client, ActiveSession::new(start_time));
    }
    fn handle_session_stop(&mut self, stop_time: OffsetDateTime, client: DestinationAddressBytes) {
        if let Some(session) = self.active_sessions.remove(&client) {
            if let Some(finished_session) = session.end_at(stop_time) {
                self.finished_sessions.push(finished_session);
            }
        }
    }

    fn handle_ecash_ticket(&mut self, ticket_type: TicketType, client: DestinationAddressBytes) {
        if let Some(active_session) = self.active_sessions.get_mut(&client) {
            if active_session.typ == SessionType::Unknown {
                active_session.set_type(ticket_type);
            }
        }
    }

    //update shared state once a day has passed, with data from the previous day
    pub(crate) async fn update_shared_state(&mut self, update_time: OffsetDateTime) {
        let update_date = update_time.date();
        if update_date != self.last_update_day {
            {
                let mut shared_state = self.shared_session_stats.write().await;
                shared_state.update_time = self.last_update_day;
                shared_state.unique_active_users = self.unique_users.len() as u32;
                shared_state.session_started = self.sessions_started;
                shared_state.sessions = self
                    .finished_sessions
                    .iter()
                    .map(|s| s.serialize())
                    .collect();
            }
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
