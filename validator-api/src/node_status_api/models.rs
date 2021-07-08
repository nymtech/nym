// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use serde::{Deserialize, Serialize};
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
    // TODO: do we even have to store 'owner' ?
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
    // TODO: do we even have to store 'owner' ?
    owner: String,

    most_recent_ipv4: bool,
    most_recent_ipv6: bool,

    // those fields really depend on how we go about implementing calculation of those values
    last_hour_ipv4: Uptime,
    last_hour_ipv6: Uptime,

    last_day_ipv4: Uptime,
    last_day_ipv6: Uptime,

    last_week_ipv4: Uptime,
    last_week_ipv6: Uptime,

    last_month_ipv4: Uptime,
    last_month_ipv6: Uptime,
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
        }
    }
}

struct Mixnode {
    id: i64,
    owner: String,
    pub_key: String,
}

struct Gateway {
    id: i64,
    owner: String,
    pub_key: String,
}

struct IpV4Status {
    timestamp: (),
    id: i64,
    up: bool,
}

struct IpV6Status {
    timestamp: (),
    id: i64,
    up: bool,
}
