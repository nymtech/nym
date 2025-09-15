// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_metrics::entry::{ActiveSession, FinishedSession};
use nym_statistics_common::types::SessionType;
use sqlx::prelude::FromRow;
use time::OffsetDateTime;

#[derive(FromRow)]
pub struct StoredFinishedSession {
    duration_ms: i64,
    typ: String,
}

impl From<StoredFinishedSession> for FinishedSession {
    fn from(value: StoredFinishedSession) -> Self {
        FinishedSession {
            duration: std::time::Duration::from_millis(value.duration_ms as u64),
            typ: SessionType::from_string(value.typ),
        }
    }
}

#[derive(FromRow)]
pub(crate) struct StoredActiveSession {
    start_time: OffsetDateTime,
    typ: String,
    remember: u8,
}

impl From<StoredActiveSession> for ActiveSession {
    fn from(value: StoredActiveSession) -> Self {
        ActiveSession {
            start: value.start_time,
            typ: SessionType::from_string(&value.typ),
            remember: value.remember != 0,
        }
    }
}
