// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_statistics_common::{hash_identifier, types::SessionType};
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Default)]
pub struct EntryStats {
    sessions: RwLock<ClientSessions>,
}

impl EntryStats {
    pub async fn update_client_sessions(&self, new: ClientSessions) {
        *self.sessions.write().await = new
    }

    pub async fn client_sessions(&self) -> RwLockReadGuard<ClientSessions> {
        self.sessions.read().await
    }
}

pub struct ClientSessions {
    pub update_time: Date,
    pub unique_users: Vec<String>,
    pub sessions_started: u32,
    pub finished_sessions: Vec<FinishedSession>,
}

impl Default for ClientSessions {
    fn default() -> Self {
        ClientSessions {
            update_time: OffsetDateTime::UNIX_EPOCH.date(),
            unique_users: vec![],
            sessions_started: 0,
            finished_sessions: vec![],
        }
    }
}

impl ClientSessions {
    pub fn new(
        update_time: Date,
        unique_users: Vec<String>,
        sessions_started: u32,
        sessions: Vec<FinishedSession>,
    ) -> Self {
        ClientSessions {
            update_time,
            unique_users: unique_users.into_iter().map(hash_identifier).collect(),
            sessions_started,
            finished_sessions: sessions,
        }
    }
}

pub struct FinishedSession {
    pub duration: Duration,
    pub typ: SessionType,
}

impl FinishedSession {
    pub fn new(duration: Duration, typ: SessionType) -> Self {
        FinishedSession { duration, typ }
    }
}

pub struct ActiveSession {
    pub start: OffsetDateTime,
    pub typ: SessionType,
}

impl ActiveSession {
    pub fn new(start_time: OffsetDateTime) -> Self {
        ActiveSession {
            start: start_time,
            typ: SessionType::Unknown,
        }
    }

    pub fn set_type(&mut self, typ: SessionType) {
        self.typ = typ;
    }

    pub fn end_at(self, stop_time: OffsetDateTime) -> Option<FinishedSession> {
        let session_duration = stop_time - self.start;
        //ensure duration is positive to fit in a u64
        //u64::max milliseconds is 500k millenia so no overflow issue
        if session_duration > Duration::ZERO {
            Some(FinishedSession {
                duration: session_duration.unsigned_abs(),
                typ: self.typ,
            })
        } else {
            None
        }
    }
}
