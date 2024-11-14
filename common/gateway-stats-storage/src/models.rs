// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_node_metrics::entry::{ActiveSession, FinishedSession, SessionType};
use sqlx::prelude::FromRow;
use time::OffsetDateTime;

pub use nym_credentials_interface::TicketType;

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

pub trait ToSessionType {
    fn to_session_type(&self) -> SessionType;
}

impl ToSessionType for TicketType {
    fn to_session_type(&self) -> SessionType {
        match self {
            TicketType::V1MixnetEntry => SessionType::Mixnet,
            TicketType::V1MixnetExit => SessionType::Mixnet,
            TicketType::V1WireguardEntry => SessionType::Vpn,
            TicketType::V1WireguardExit => SessionType::Vpn,
        }
    }
}

#[derive(FromRow)]
pub(crate) struct StoredActiveSession {
    start_time: OffsetDateTime,
    typ: String,
}

impl From<StoredActiveSession> for ActiveSession {
    fn from(value: StoredActiveSession) -> Self {
        ActiveSession {
            start: value.start_time,
            typ: SessionType::from_string(&value.typ),
        }
    }
}
