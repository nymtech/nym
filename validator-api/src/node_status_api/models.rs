// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::{FIFTEEN_MINUTES, ONE_HOUR};
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::io::Cursor;

// something like enum uptime (Uptime, NoUptime) etc

// todo: put into some error enum
#[derive(Debug)]
pub struct InvalidUptime;

// value in range 0-100
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Uptime(u8);

impl Uptime {
    pub const fn zero() -> Self {
        Uptime(0)
    }

    pub fn from_ratio(numerator: usize, denominator: usize) -> Result<Self, InvalidUptime> {
        if denominator == 0 {
            return Ok(Self::zero());
        }

        let uptime = ((numerator as f32 / denominator as f32) * 100.0) as u8;

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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MixnodeStatusReport {
    identity: String,
    owner: String,

    most_recent_ipv4: bool,
    most_recent_ipv6: bool,

    // those fields really depend on how we go about implementing calculation of those values
    last_hour_ipv4: Uptime,
    last_hour_ipv6: Uptime,

    last_day_ipv4: Uptime,
    last_day_ipv6: Uptime,
}

impl MixnodeStatusReport {
    pub(crate) fn construct_from_last_day_reports(
        identity: &str,
        last_day_ipv4: Vec<StatusReport>,
        last_day_ipv6: Vec<StatusReport>,
    ) -> Self {
        let now = OffsetDateTime::now_utc();
        let hour_ago = (now - ONE_HOUR).unix_timestamp();
        let fifteen_minutes_ago = (now - FIFTEEN_MINUTES).unix_timestamp();

        let ipv4_day_total = last_day_ipv4.len();
        let ipv6_day_total = last_day_ipv6.len();

        let ipv4_day_up = last_day_ipv4.iter().filter(|report| report.up).count();
        let ipv6_day_up = last_day_ipv6.iter().filter(|report| report.up).count();

        let ipv4_hour_total = last_day_ipv4
            .iter()
            .filter(|report| report.timestamp >= hour_ago)
            .count();
        let ipv6_hour_total = last_day_ipv6
            .iter()
            .filter(|report| report.timestamp >= hour_ago)
            .count();

        let ipv4_hour_up = last_day_ipv4
            .iter()
            .filter(|report| report.up && report.timestamp >= hour_ago)
            .count();
        let ipv6_hour_up = last_day_ipv6
            .iter()
            .filter(|report| report.up && report.timestamp >= hour_ago)
            .count();

        // most recent status MUST BE within last 15min
        let most_recent_ipv4 = last_day_ipv4
            .iter()
            .max_by_key(|report| report.timestamp) // find the most recent
            .map(|status| status.timestamp >= fifteen_minutes_ago && status.up) // make sure its within last 15min
            .unwrap_or_default();
        let most_recent_ipv6 = last_day_ipv6
            .iter()
            .max_by_key(|report| report.timestamp) // find the most recent
            .map(|status| status.timestamp >= fifteen_minutes_ago && status.up) // make sure its within last 15min
            .unwrap_or_default();

        // the unwraps in Uptime::from_ratio are fine because it's impossible for us to have more "up" results than all results in total
        // because both of those values originate from the same vector
        MixnodeStatusReport {
            identity: identity.to_owned(),
            owner: "TODO: grab that data somehow... somewhere...".to_string(),
            most_recent_ipv4,
            most_recent_ipv6,
            last_hour_ipv4: Uptime::from_ratio(ipv4_hour_up, ipv4_hour_total).unwrap(),
            last_hour_ipv6: Uptime::from_ratio(ipv6_hour_up, ipv6_hour_total).unwrap(),
            last_day_ipv4: Uptime::from_ratio(ipv4_day_up, ipv4_day_total).unwrap(),
            last_day_ipv6: Uptime::from_ratio(ipv6_day_up, ipv6_day_total).unwrap(),
        }
    }

    pub fn example() -> Self {
        MixnodeStatusReport {
            identity: "aaaa".to_string(),
            owner: "bbbb".to_string(),
            most_recent_ipv4: false,
            most_recent_ipv6: false,
            last_hour_ipv4: Uptime(42),
            last_hour_ipv6: Uptime(0),
            last_day_ipv4: Uptime(12),
            last_day_ipv6: Uptime(12),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GatewayStatusReport {
    identity: String,
    owner: String,

    most_recent_ipv4: bool,
    most_recent_ipv6: bool,

    last_hour_ipv4: Uptime,
    last_hour_ipv6: Uptime,

    last_day_ipv4: Uptime,
    last_day_ipv6: Uptime,
}

// Internally used struct to catch results from the database to calculate uptimes for given mixnode/gateway
pub(crate) struct StatusReport {
    pub(crate) timestamp: i64,
    pub(crate) up: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MixnodeUptimeHistory {
    identity: String,
    owner: String,

    history: Vec<HistoricalUptime>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct GatewayUptimeHistory {
    identity: String,
    owner: String,

    history: Vec<HistoricalUptime>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct HistoricalUptime {
    // ISO 8601 date string
    // I think this is more than enough, we don't need the uber precision of timezone offsets, etc
    date: String,

    uptime: Uptime,
}

pub(crate) struct ErrorResponse {
    error: NodeStatusApiError,
    status: Status,
}

impl ErrorResponse {
    pub(crate) fn new(error: NodeStatusApiError, status: Status) -> Self {
        ErrorResponse { error, status }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ErrorResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let message = format!("{}", self.error);
        Response::build()
            .header(ContentType::Plain)
            .sized_body(message.len(), Cursor::new(message))
            .status(self.status)
            .ok()
    }
}

#[derive(Debug)]
pub enum NodeStatusApiError {
    MixnodeReportNotFound(String),
    GatewayReportNotFound(String),

    // I don't think we want to expose errors to the user about what really happened
    InternalDatabaseError,
}

impl Display for NodeStatusApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NodeStatusApiError::MixnodeReportNotFound(identity) => write!(
                f,
                "Could not find status report associated with mixnode {}",
                identity
            ),
            NodeStatusApiError::GatewayReportNotFound(identity) => write!(
                f,
                "Could not find status report associated with gateway {}",
                identity
            ),
            NodeStatusApiError::InternalDatabaseError => {
                write!(f, "The internal database has experienced an issue")
            }
        }
    }
}
