// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::io::Cursor;

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

    last_week_ipv4: Uptime,
    last_week_ipv6: Uptime,

    last_month_ipv4: Uptime,
    last_month_ipv6: Uptime,
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
            last_week_ipv4: Uptime(12),
            last_week_ipv6: Uptime(12),
            last_month_ipv4: Uptime(12),
            last_month_ipv6: Uptime(12),
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

pub(crate) struct ErrorResponseNew {
    error: NodeStatusApiError,
    status: Status,
}

impl ErrorResponseNew {
    pub(crate) fn new(error: NodeStatusApiError, status: Status) -> Self {
        ErrorResponseNew { error, status }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ErrorResponseNew {
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

// OLD RELATING TO NODE_STATUS_API CLIENT USED BY MONITOR

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given mix is
/// currently up or down (based on whether it's mixing packets)
pub struct MixStatus {
    pub pub_key: String,
    pub owner: String,
    pub ip_version: String,
    pub up: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given set of mixes is
/// currently up or down (based on whether it's mixing packets)
pub struct BatchMixStatus {
    pub status: Vec<MixStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given gateway is
/// currently up or down (based on whether it's mixing packets)
pub struct GatewayStatus {
    pub pub_key: String,
    pub owner: String,
    pub ip_version: String,
    pub up: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
/// A notification sent to the validators to let them know whether a given set of gateways is
/// currently up or down (based on whether it's mixing packets)
pub struct BatchGatewayStatus {
    pub status: Vec<GatewayStatus>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum ErrorResponses {
    Error(ErrorResponse),
    Unexpected(serde_json::Value),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OkResponse {
    pub(crate) ok: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub(crate) enum DefaultRestResponse {
    Ok(OkResponse),
    Error(ErrorResponses),
}
