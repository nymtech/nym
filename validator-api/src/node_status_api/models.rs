// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::utils::NodeUptimes;
use crate::storage::models::NodeStatus;
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::io::Cursor;
use time::OffsetDateTime;

// todo: put into some error enum
#[derive(Debug)]
pub struct InvalidUptime;

// value in range 0-100
#[derive(Clone, Copy, Serialize, Deserialize, Debug, Default)]
pub struct Uptime(u8);

impl Uptime {
    pub const fn zero() -> Self {
        Uptime(0)
    }

    pub fn new(uptime: f32) -> Self {
        if uptime > 100f32 {
            error!("Got uptime {}, max is 100, returning 0", uptime);
            Uptime(0)
        } else {
            Uptime(uptime as u8)
        }
    }

    pub fn from_ratio(numerator: usize, denominator: usize) -> Result<Self, InvalidUptime> {
        if denominator == 0 {
            return Ok(Self::zero());
        }

        let uptime = ((numerator as f32 / denominator as f32) * 100.0).round() as u8;

        if uptime > 100 {
            Err(InvalidUptime)
        } else {
            Ok(Uptime(uptime))
        }
    }

    pub fn from_uptime_sum(running_sum: f32, count: usize) -> Result<Self, InvalidUptime> {
        if count == 0 {
            return Ok(Self::zero());
        }

        let uptime = (running_sum / count as f32).round() as u8;

        if uptime > 100 {
            Err(InvalidUptime)
        } else {
            Ok(Uptime(uptime))
        }
    }

    pub fn u8(&self) -> u8 {
        self.0
    }
}

impl From<Uptime> for u8 {
    fn from(uptime: Uptime) -> Self {
        uptime.0
    }
}

impl TryFrom<u8> for Uptime {
    type Error = InvalidUptime;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 100 {
            Err(InvalidUptime)
        } else {
            Ok(Uptime(value))
        }
    }
}

impl TryFrom<i64> for Uptime {
    type Error = InvalidUptime;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if !(0..=100).contains(&value) {
            Err(InvalidUptime)
        } else {
            Ok(Uptime(value as u8))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MixnodeStatusReport {
    pub(crate) identity: String,
    pub(crate) owner: String,

    pub(crate) most_recent: Uptime,

    pub(crate) last_hour: Uptime,
    pub(crate) last_day: Uptime,
}

impl MixnodeStatusReport {
    pub(crate) fn construct_from_last_day_reports(
        report_time: OffsetDateTime,
        identity: String,
        owner: String,
        last_day: Vec<NodeStatus>,
        last_hour_test_runs: usize,
        last_day_test_runs: usize,
    ) -> Self {
        let node_uptimes = NodeUptimes::calculate_from_last_day_reports(
            report_time,
            last_day,
            last_hour_test_runs,
            last_day_test_runs,
        );

        MixnodeStatusReport {
            identity,
            owner,
            most_recent: node_uptimes.most_recent,
            last_hour: node_uptimes.last_hour,
            last_day: node_uptimes.last_day,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GatewayStatusReport {
    pub(crate) identity: String,
    pub(crate) owner: String,

    pub(crate) most_recent: Uptime,

    pub(crate) last_hour: Uptime,
    pub(crate) last_day: Uptime,
}

impl GatewayStatusReport {
    pub(crate) fn construct_from_last_day_reports(
        report_time: OffsetDateTime,
        identity: String,
        owner: String,
        last_day: Vec<NodeStatus>,
        last_hour_test_runs: usize,
        last_day_test_runs: usize,
    ) -> Self {
        let node_uptimes = NodeUptimes::calculate_from_last_day_reports(
            report_time,
            last_day,
            last_hour_test_runs,
            last_day_test_runs,
        );

        GatewayStatusReport {
            identity,
            owner,
            most_recent: node_uptimes.most_recent,
            last_hour: node_uptimes.last_hour,
            last_day: node_uptimes.last_day,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MixnodeUptimeHistory {
    pub(crate) identity: String,
    pub(crate) owner: String,

    pub(crate) history: Vec<HistoricalUptime>,
}

impl MixnodeUptimeHistory {
    pub(crate) fn new(identity: String, owner: String, history: Vec<HistoricalUptime>) -> Self {
        MixnodeUptimeHistory {
            identity,
            owner,
            history,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GatewayUptimeHistory {
    pub(crate) identity: String,
    pub(crate) owner: String,

    pub(crate) history: Vec<HistoricalUptime>,
}

impl GatewayUptimeHistory {
    pub(crate) fn new(identity: String, owner: String, history: Vec<HistoricalUptime>) -> Self {
        GatewayUptimeHistory {
            identity,
            owner,
            history,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct HistoricalUptime {
    // ISO 8601 date string
    // I think this is more than enough, we don't need the uber precision of timezone offsets, etc
    pub(crate) date: String,

    pub(crate) uptime: Uptime,
}

pub(crate) struct ErrorResponse {
    error_message: String,
    status: Status,
}

impl ErrorResponse {
    pub(crate) fn new(error_message: impl Into<String>, status: Status) -> Self {
        ErrorResponse {
            error_message: error_message.into(),
            status,
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ErrorResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        Response::build()
            .header(ContentType::Plain)
            .sized_body(self.error_message.len(), Cursor::new(self.error_message))
            .status(self.status)
            .ok()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidatorApiStorageError {
    MixnodeReportNotFound(String),
    GatewayReportNotFound(String),
    MixnodeUptimeHistoryNotFound(String),
    GatewayUptimeHistoryNotFound(String),

    // I don't think we want to expose errors to the user about what really happened
    InternalDatabaseError(String),
}

impl Display for ValidatorApiStorageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ValidatorApiStorageError::MixnodeReportNotFound(identity) => write!(
                f,
                "Could not find status report associated with mixnode {}",
                identity
            ),
            ValidatorApiStorageError::GatewayReportNotFound(identity) => write!(
                f,
                "Could not find status report associated with gateway {}",
                identity
            ),
            ValidatorApiStorageError::MixnodeUptimeHistoryNotFound(identity) => write!(
                f,
                "Could not find uptime history associated with mixnode {}",
                identity
            ),
            ValidatorApiStorageError::GatewayUptimeHistoryNotFound(identity) => write!(
                f,
                "Could not find uptime history associated with gateway {}",
                identity
            ),
            ValidatorApiStorageError::InternalDatabaseError(err) => {
                write!(f, "The internal database has experienced an issue: {err}")
            }
        }
    }
}
