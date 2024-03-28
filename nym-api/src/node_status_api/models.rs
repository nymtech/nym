// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::{EcashError, RedemptionError};
use crate::node_status_api::utils::NodeUptimes;
use crate::storage::models::NodeStatus;
use crate::support::caching::cache::UninitialisedCache;
use nym_api_requests::models::{
    HistoricalPerformanceResponse, HistoricalUptimeResponse, NodePerformance,
    OldHistoricalUptimeResponse, RequestError,
};
use nym_contracts_common::NaiveFloat;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{IdentityKey, NodeId};
use nym_serde_helpers::date::DATE_FORMAT;
use reqwest::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;
use time::{Date, OffsetDateTime};
use tracing::error;

#[derive(Error, Debug)]
#[error("Received uptime value was within 0-100 range (got {received})")]
pub struct InvalidUptime {
    received: isize,
}

// value in range 0-100
#[derive(Clone, Copy, Serialize, Deserialize, Debug, Default, JsonSchema)]
pub struct Uptime(u8);

impl Uptime {
    pub const fn zero() -> Self {
        Uptime(0)
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
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
            Err(InvalidUptime {
                received: uptime as isize,
            })
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
            Err(InvalidUptime {
                received: uptime as isize,
            })
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
            Err(InvalidUptime {
                received: value as isize,
            })
        } else {
            Ok(Uptime(value))
        }
    }
}

impl TryFrom<i64> for Uptime {
    type Error = InvalidUptime;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if !(0..=100).contains(&value) {
            Err(InvalidUptime {
                received: value as isize,
            })
        } else {
            Ok(Uptime(value as u8))
        }
    }
}

impl From<Uptime> for Performance {
    fn from(uptime: Uptime) -> Self {
        Performance::from_percentage_value(uptime.0 as u64).unwrap()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct MixnodeStatusReport {
    pub(crate) mix_id: NodeId,
    pub(crate) identity: IdentityKey,

    pub(crate) most_recent: Uptime,

    pub(crate) last_hour: Uptime,
    pub(crate) last_day: Uptime,
}

impl MixnodeStatusReport {
    pub(crate) fn construct_from_last_day_reports(
        report_time: OffsetDateTime,
        mix_id: NodeId,
        identity: IdentityKey,
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
            mix_id,
            identity,
            most_recent: node_uptimes.most_recent,
            last_hour: node_uptimes.last_hour,
            last_day: node_uptimes.last_day,
        }
    }
}

impl From<MixnodeStatusReport> for NodePerformance {
    fn from(report: MixnodeStatusReport) -> Self {
        NodePerformance {
            most_recent: report.most_recent.into(),
            last_hour: report.last_hour.into(),
            last_24h: report.last_day.into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct GatewayStatusReport {
    pub(crate) node_id: NodeId,
    pub(crate) identity: String,

    pub(crate) most_recent: Uptime,

    pub(crate) last_hour: Uptime,
    pub(crate) last_day: Uptime,
}

impl GatewayStatusReport {
    pub(crate) fn construct_from_last_day_reports(
        report_time: OffsetDateTime,
        node_id: NodeId,
        identity: String,
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
            node_id,
            most_recent: node_uptimes.most_recent,
            last_hour: node_uptimes.last_hour,
            last_day: node_uptimes.last_day,
        }
    }
}

impl From<GatewayStatusReport> for NodePerformance {
    fn from(report: GatewayStatusReport) -> Self {
        NodePerformance {
            most_recent: report.most_recent.into(),
            last_hour: report.last_hour.into(),
            last_24h: report.last_day.into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct MixnodeUptimeHistory {
    pub(crate) mix_id: NodeId,
    pub(crate) identity: String,

    pub(crate) history: Vec<HistoricalUptime>,
}

impl MixnodeUptimeHistory {
    pub(crate) fn new(mix_id: NodeId, identity: String, history: Vec<HistoricalUptime>) -> Self {
        MixnodeUptimeHistory {
            mix_id,
            identity,
            history,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct GatewayUptimeHistory {
    pub(crate) identity: String,
    pub(crate) node_id: NodeId,

    pub(crate) history: Vec<HistoricalUptime>,
}

impl GatewayUptimeHistory {
    pub(crate) fn new(
        node_id: NodeId,
        identity: impl Into<String>,
        history: Vec<HistoricalUptime>,
    ) -> Self {
        GatewayUptimeHistory {
            node_id,
            identity: identity.into(),
            history,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct HistoricalUptime {
    // ISO 8601 date string
    // I think this is more than enough, we don't need the uber precision of timezone offsets, etc
    pub(crate) date: String,

    pub(crate) uptime: Uptime,
}

#[derive(Error, Debug)]
pub enum InvalidHistoricalPerformance {
    #[error("the provided date could not be parsed")]
    UnparsableDate,

    #[error("the provided uptime could not be parsed")]
    MalformedPerformance,
}

impl TryFrom<HistoricalUptime> for HistoricalPerformanceResponse {
    type Error = InvalidHistoricalPerformance;
    fn try_from(value: HistoricalUptime) -> Result<Self, Self::Error> {
        Ok(HistoricalPerformanceResponse {
            date: Date::parse(&value.date, DATE_FORMAT)
                .map_err(|_| InvalidHistoricalPerformance::UnparsableDate)?,
            performance: Performance::from_percentage_value(value.uptime.u8() as u64)
                .map_err(|_| InvalidHistoricalPerformance::MalformedPerformance)?
                .naive_to_f64(),
        })
    }
}

impl TryFrom<HistoricalUptime> for HistoricalUptimeResponse {
    type Error = InvalidHistoricalPerformance;
    fn try_from(value: HistoricalUptime) -> Result<Self, Self::Error> {
        Ok(HistoricalUptimeResponse {
            date: Date::parse(&value.date, DATE_FORMAT)
                .map_err(|_| InvalidHistoricalPerformance::UnparsableDate)?,
            uptime: value.uptime.u8(),
        })
    }
}

impl From<HistoricalUptime> for OldHistoricalUptimeResponse {
    fn from(uptime: HistoricalUptime) -> Self {
        OldHistoricalUptimeResponse {
            date: uptime.date,
            uptime: uptime.uptime.0,
        }
    }
}

// TODO rocket remove smurf name after eliminating `rocket`
pub(crate) type AxumResult<T> = Result<T, AxumErrorResponse>;
pub(crate) struct AxumErrorResponse {
    message: RequestError,
    status: StatusCode,
}

impl AxumErrorResponse {
    pub(crate) fn internal_msg(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn internal() -> Self {
        Self {
            message: RequestError::new("Internal server error"),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn not_implemented() -> Self {
        Self {
            message: RequestError::empty(),
            status: StatusCode::NOT_IMPLEMENTED,
        }
    }

    pub(crate) fn not_found(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub(crate) fn service_unavailable() -> Self {
        Self {
            message: RequestError::empty(),
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub(crate) fn unprocessable_entity(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    pub(crate) fn forbidden(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: StatusCode::FORBIDDEN,
        }
    }

    pub(crate) fn bad_request(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

impl From<UninitialisedCache> for AxumErrorResponse {
    fn from(_: UninitialisedCache) -> Self {
        AxumErrorResponse {
            message: RequestError::new("relevant cache hasn't been initialised yet"),
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl axum::response::IntoResponse for AxumErrorResponse {
    fn into_response(self) -> axum::response::Response {
        (self.status, self.message.message().to_string()).into_response()
    }
}

impl From<NymApiStorageError> for AxumErrorResponse {
    fn from(value: NymApiStorageError) -> Self {
        error!("{value}");
        Self {
            message: RequestError::empty(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<EcashError> for AxumErrorResponse {
    fn from(value: EcashError) -> Self {
        Self {
            message: RequestError::new(value.to_string()),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

impl From<RedemptionError> for AxumErrorResponse {
    fn from(value: RedemptionError) -> Self {
        Self {
            message: RequestError::new(value.to_string()),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

#[derive(Debug, Error)]
pub enum NymApiStorageError {
    #[error("could not find status report associated with mixnode {mix_id}")]
    MixnodeReportNotFound { mix_id: NodeId },

    #[error("Could not find status report associated with gateway {node_id}")]
    GatewayReportNotFound { node_id: NodeId },

    #[error("could not find uptime history associated with mixnode {mix_id}")]
    MixnodeUptimeHistoryNotFound { mix_id: NodeId },

    #[error("could not find uptime history associated with gateway {node_id}")]
    GatewayUptimeHistoryNotFound { node_id: NodeId },

    #[error("could not find gateway {identity} in the storage")]
    GatewayNotFound { identity: String },

    // I don't think we want to expose errors to the user about what really happened
    #[error("experienced internal database error")]
    InternalDatabaseError(#[from] sqlx::Error),

    // the same is true here (also note that the message is subtly different so we would be able to distinguish them)
    #[error("experienced internal storage error")]
    DatabaseInconsistency { reason: String },

    // this one would never be returned to users since it's only possible on startup
    #[error("failed to perform startup SQL migration - {0}")]
    StartupMigrationFailure(#[from] sqlx::migrate::MigrateError),
}

impl NymApiStorageError {
    pub fn database_inconsistency<S: Into<String>>(reason: S) -> NymApiStorageError {
        NymApiStorageError::DatabaseInconsistency {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_response_conversion() {}
}
