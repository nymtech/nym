// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::utils::NodeUptimes;
use crate::storage::models::NodeStatus;
use nym_api_requests::models::{
    GatewayStatusReportResponse, GatewayUptimeHistoryResponse, HistoricalUptimeResponse,
    MixnodeStatusReportResponse, MixnodeUptimeHistoryResponse, NodePerformance, RequestError,
};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{IdentityKey, MixId};
use okapi::openapi3::{Responses, SchemaObject};
use rocket::http::Status;
use rocket::response::{self, Responder, Response};
use rocket::serde::json::Json;
use rocket::Request;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::util::ensure_status_code_exists;
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use thiserror::Error;
use time::OffsetDateTime;

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
    pub(crate) mix_id: MixId,
    pub(crate) identity: IdentityKey,
    pub(crate) owner: String,

    pub(crate) most_recent: Uptime,

    pub(crate) last_hour: Uptime,
    pub(crate) last_day: Uptime,
}

impl MixnodeStatusReport {
    pub(crate) fn construct_from_last_day_reports(
        report_time: OffsetDateTime,
        mix_id: MixId,
        identity: IdentityKey,
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
            mix_id,
            identity,
            owner,
            most_recent: node_uptimes.most_recent,
            last_hour: node_uptimes.last_hour,
            last_day: node_uptimes.last_day,
        }
    }
}

impl From<MixnodeStatusReport> for MixnodeStatusReportResponse {
    fn from(status: MixnodeStatusReport) -> Self {
        MixnodeStatusReportResponse {
            mix_id: status.mix_id,
            identity: status.identity,
            owner: status.owner,
            most_recent: status.most_recent.0,
            last_hour: status.last_hour.0,
            last_day: status.last_day.0,
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

impl From<GatewayStatusReport> for GatewayStatusReportResponse {
    fn from(status: GatewayStatusReport) -> Self {
        GatewayStatusReportResponse {
            identity: status.identity,
            owner: status.owner,
            most_recent: status.most_recent.0,
            last_hour: status.last_hour.0,
            last_day: status.last_day.0,
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
    pub(crate) mix_id: MixId,
    pub(crate) identity: String,
    pub(crate) owner: String,

    pub(crate) history: Vec<HistoricalUptime>,
}

impl MixnodeUptimeHistory {
    pub(crate) fn new(
        mix_id: MixId,
        identity: String,
        owner: String,
        history: Vec<HistoricalUptime>,
    ) -> Self {
        MixnodeUptimeHistory {
            mix_id,
            identity,
            owner,
            history,
        }
    }
}

impl From<MixnodeUptimeHistory> for MixnodeUptimeHistoryResponse {
    fn from(history: MixnodeUptimeHistory) -> Self {
        MixnodeUptimeHistoryResponse {
            mix_id: history.mix_id,
            identity: history.identity,
            owner: history.owner,
            history: history.history.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
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

impl From<GatewayUptimeHistory> for GatewayUptimeHistoryResponse {
    fn from(history: GatewayUptimeHistory) -> Self {
        GatewayUptimeHistoryResponse {
            identity: history.identity,
            owner: history.owner,
            history: history.history.into_iter().map(Into::into).collect(),
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

impl From<HistoricalUptime> for HistoricalUptimeResponse {
    fn from(uptime: HistoricalUptime) -> Self {
        HistoricalUptimeResponse {
            date: uptime.date,
            uptime: uptime.uptime.0,
        }
    }
}

#[deprecated(note = "TODO dz remove once Rocket is phased out")]
pub(crate) struct RocketErrorResponse {
    error_message: RequestError,
    status: Status,
}

impl RocketErrorResponse {
    pub(crate) fn new(error_message: impl Into<String>, status: Status) -> Self {
        RocketErrorResponse {
            error_message: RequestError::new(error_message),
            status,
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for RocketErrorResponse {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        // piggyback on the existing implementation
        // also prefer json over plain for ease of use in frontend
        Response::build()
            .merge(Json(self.error_message).respond_to(req)?)
            .status(self.status)
            .ok()
    }
}

impl JsonSchema for RocketErrorResponse {
    fn schema_name() -> String {
        "ErrorResponse".to_owned()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        let mut schema_object = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            ..SchemaObject::default()
        };

        let object_validation = schema_object.object();
        object_validation
            .properties
            .insert("error_message".to_owned(), gen.subschema_for::<String>());
        object_validation
            .required
            .insert("error_message".to_owned());

        // Status does not implement JsonSchema so we just explicitly specify the inner type.
        object_validation
            .properties
            .insert("status".to_owned(), gen.subschema_for::<u16>());
        object_validation.required.insert("status".to_owned());

        Schema::Object(schema_object)
    }
}

impl OpenApiResponderInner for RocketErrorResponse {
    fn responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        let mut responses = Responses::default();
        ensure_status_code_exists(&mut responses, 404);
        Ok(responses)
    }
}

pub(crate) type AxumResult<T> = Result<T, AxumErrorResponse>;

// TODO dz remove smurf name after eliminating `rocket`
pub(crate) struct AxumErrorResponse {
    message: RequestError,
    status: axum::http::StatusCode,
}

impl AxumErrorResponse {
    pub(crate) fn internal_msg(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn internal() -> Self {
        Self {
            message: RequestError::new("Internal server error"),
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn not_implemented() -> Self {
        Self {
            message: RequestError::empty(),
            status: axum::http::StatusCode::NOT_IMPLEMENTED,
        }
    }

    pub(crate) fn not_found(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: axum::http::StatusCode::NOT_FOUND,
        }
    }

    pub(crate) fn service_unavailable() -> Self {
        Self {
            message: RequestError::empty(),
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub(crate) fn unprocessable_entity(msg: impl Display) -> Self {
        Self {
            message: RequestError::new(msg.to_string()),
            status: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
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
            status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NymApiStorageError {
    #[error("could not find status report associated with mixnode {mix_id}")]
    MixnodeReportNotFound { mix_id: MixId },

    #[error("Could not find status report associated with gateway {identity}")]
    GatewayReportNotFound { identity: IdentityKey },

    #[error("could not find uptime history associated with mixnode {mix_id}")]
    MixnodeUptimeHistoryNotFound { mix_id: MixId },

    #[error("could not find uptime history associated with gateway {identity}")]
    GatewayUptimeHistoryNotFound { identity: IdentityKey },

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
