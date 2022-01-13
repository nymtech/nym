// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    ErrorResponse, GatewayStatusReport, GatewayUptimeHistory, MixnodeStatusReport,
    MixnodeUptimeHistory,
};
use crate::storage::ValidatorApiStorage;
use crate::{Epoch, ValidatorCache};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use std::convert::TryFrom;
use time::OffsetDateTime;
use validator_api_requests::models::{
    CoreNodeStatusResponse, MixnodeStatusResponse, RewardEstimationResponse,
    StakeSaturationResponse,
};

#[get("/mixnode/<identity>/report")]
pub(crate) async fn mixnode_report(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<MixnodeStatusReport>, ErrorResponse> {
    storage
        .construct_mixnode_report(identity)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[get("/gateway/<identity>/report")]
pub(crate) async fn gateway_report(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<GatewayStatusReport>, ErrorResponse> {
    storage
        .construct_gateway_report(identity)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[get("/mixnode/<identity>/history")]
pub(crate) async fn mixnode_uptime_history(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<MixnodeUptimeHistory>, ErrorResponse> {
    storage
        .get_mixnode_uptime_history(identity)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[get("/gateway/<identity>/history")]
pub(crate) async fn gateway_uptime_history(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<GatewayUptimeHistory>, ErrorResponse> {
    storage
        .get_gateway_uptime_history(identity)
        .await
        .map(Json)
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))
}

#[get("/mixnode/<identity>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Json<CoreNodeStatusResponse> {
    let count = storage
        .get_core_mixnode_status_count(identity, since)
        .await
        .unwrap_or_default();

    Json(CoreNodeStatusResponse {
        identity: identity.to_string(),
        count,
    })
}

#[get("/gateway/<identity>/core-status-count?<since>")]
pub(crate) async fn gateway_core_status_count(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Json<CoreNodeStatusResponse> {
    let count = storage
        .get_core_gateway_status_count(identity, since)
        .await
        .unwrap_or_default();

    Json(CoreNodeStatusResponse {
        identity: identity.to_string(),
        count,
    })
}

#[get("/mixnode/<identity>/status")]
pub(crate) async fn get_mixnode_status(
    cache: &State<ValidatorCache>,
    identity: String,
) -> Json<MixnodeStatusResponse> {
    Json(MixnodeStatusResponse {
        status: cache.mixnode_status(identity).await,
    })
}

#[get("/mixnode/<identity>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    first_epoch: &State<Epoch>,
    identity: String,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    let (bond, status) = cache.mixnode_details(&identity).await;
    if let Some(bond) = bond {
        let epoch_reward_params = cache.epoch_reward_params().await;
        let as_at = epoch_reward_params.timestamp();
        let epoch_reward_params = epoch_reward_params.into_inner();

        let current_epoch = first_epoch.current(OffsetDateTime::now_utc());
        let uptime = storage
            .get_average_mixnode_uptime_in_interval(
                &identity,
                current_epoch.start_unix_timestamp(),
                current_epoch.end_unix_timestamp(),
            )
            .await
            .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))?;

        let (estimated_total_node_reward, estimated_operator_reward, estimated_delegators_reward) =
            epoch_reward_params.estimate_reward(&bond, uptime.u8(), status.is_active());

        Ok(Json(RewardEstimationResponse {
            estimated_total_node_reward: estimated_total_node_reward as u64,
            estimated_operator_reward: estimated_operator_reward as u64,
            estimated_delegators_reward: estimated_delegators_reward as u64,
            current_epoch_start: current_epoch.start_unix_timestamp(),
            current_epoch_end: current_epoch.end_unix_timestamp(),
            current_epoch_uptime: uptime.u8(),
            as_at,
        }))
    } else {
        Err(ErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}

#[get("/mixnode/<identity>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation(
    cache: &State<ValidatorCache>,
    identity: String,
) -> Result<Json<StakeSaturationResponse>, ErrorResponse> {
    let (bond, _) = cache.mixnode_details(&identity).await;
    if let Some(bond) = bond {
        let epoch_reward_params = cache.epoch_reward_params().await;
        let as_at = epoch_reward_params.timestamp();
        let epoch_reward_params = epoch_reward_params.into_inner();

        let saturation = bond.stake_saturation(
            epoch_reward_params.circulating_supply,
            epoch_reward_params.rewarded_set_size,
        );

        Ok(Json(StakeSaturationResponse {
            saturation: saturation.to_num(),
            as_at,
        }))
    } else {
        Err(ErrorResponse::new(
            "mixnode bond not found",
            Status::NotFound,
        ))
    }
}
