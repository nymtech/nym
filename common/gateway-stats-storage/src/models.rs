// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials_interface::TicketType;
use sqlx::prelude::FromRow;
use time::{Duration, OffsetDateTime};

#[derive(FromRow)]
pub struct StoredFinishedSession {
    duration_ms: i64,
    typ: String,
}

impl StoredFinishedSession {
    pub fn serialize(&self) -> (u64, String) {
        (
            self.duration_ms as u64, //we are sure that it fits in a u64, see `fn end_at`
            self.typ.clone(),
        )
    }
}

pub struct FinishedSession {
    pub duration: Duration,
    pub typ: SessionType,
}

#[derive(PartialEq)]
pub enum SessionType {
    Vpn,
    Mixnet,
    Unknown,
}

impl SessionType {
    pub fn to_string(&self) -> &str {
        match self {
            Self::Vpn => "vpn",
            Self::Mixnet => "mixnet",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "vpn" => Self::Vpn,
            "mixnet" => Self::Mixnet,
            _ => Self::Unknown,
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

#[derive(FromRow)]
pub(crate) struct StoredActiveSession {
    start_time: OffsetDateTime,
    typ: String,
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

    pub fn set_type(&mut self, ticket_type: TicketType) {
        self.typ = ticket_type.into();
    }

    pub fn end_at(self, stop_time: OffsetDateTime) -> Option<FinishedSession> {
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

impl From<StoredActiveSession> for ActiveSession {
    fn from(value: StoredActiveSession) -> Self {
        ActiveSession {
            start: value.start_time,
            typ: SessionType::from_string(&value.typ),
        }
    }
}
