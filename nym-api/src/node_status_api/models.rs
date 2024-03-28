// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::utils::NodeUptimes;
use crate::storage::models::NodeStatus;
use nym_api_requests::models::{HistoricalUptimeResponse, NodePerformance, RequestError};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{IdentityKey, NodeId};
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

use crate::support::caching::cache::UninitialisedCache;
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

impl From<HistoricalUptime> for HistoricalUptimeResponse {
    fn from(uptime: HistoricalUptime) -> Self {
        HistoricalUptimeResponse {
            date: uptime.date,
            uptime: uptime.uptime.0,
        }
    }
}

pub(crate) struct ErrorResponse {
    error_message: RequestError,
    status: Status,
}

impl ErrorResponse {
    pub(crate) fn new(error_message: impl Into<String>, status: Status) -> Self {
        ErrorResponse {
            error_message: RequestError::new(error_message),
            status,
        }
    }

    pub(crate) fn internal_server_error() -> Self {
        ErrorResponse {
            error_message: RequestError::new(
                "experienced an internal server error and could not complete this request",
            ),
            status: Status::InternalServerError,
        }
    }
}

impl From<UninitialisedCache> for ErrorResponse {
    fn from(_: UninitialisedCache) -> Self {
        ErrorResponse {
            error_message: RequestError::new(
                "one of the internal caches hasn't yet been initialised",
            ),
            status: Status::ServiceUnavailable,
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ErrorResponse {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        // piggyback on the existing implementation
        // also prefer json over plain for ease of use in frontend
        Response::build()
            .merge(Json(self.error_message).respond_to(req)?)
            .status(self.status)
            .ok()
    }
}

impl JsonSchema for ErrorResponse {
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

impl OpenApiResponderInner for ErrorResponse {
    fn responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        let mut responses = Responses::default();
        ensure_status_code_exists(&mut responses, 404);
        Ok(responses)
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
