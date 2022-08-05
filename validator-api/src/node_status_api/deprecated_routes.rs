// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::helpers::{
    _compute_mixnode_reward_estimation, _get_mixnode_avg_uptime,
    _get_mixnode_inclusion_probability, _get_mixnode_reward_estimation,
    _get_mixnode_stake_saturation, _get_mixnode_status, _mixnode_core_status_count,
    _mixnode_report, _mixnode_uptime_history,
};
use crate::node_status_api::models::{ErrorResponse, MixnodeStatusReport, MixnodeUptimeHistory};
use crate::{ValidatorApiStorage, ValidatorCache};
use mixnet_contract_common::NodeId;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator_api_requests::models::{
    DeprecatedComputeRewardEstParam, DeprecatedRewardEstimationResponse, DeprecatedUptimeResponse,
    InclusionProbabilityResponse, MixnodeCoreStatusResponse, MixnodeStatus, MixnodeStatusResponse,
    StakeSaturationResponse,
};

pub trait Deprecatable {
    fn deprecate(self) -> Deprecated<Self>
    where
        Self: Sized,
    {
        self.into()
    }
}

impl<T> Deprecatable for T {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Deprecated<T> {
    deprecated: bool,
    #[serde(flatten)]
    response: T,
}

impl<T> From<T> for Deprecated<T> {
    fn from(response: T) -> Self {
        Deprecated {
            deprecated: true,
            response,
        }
    }
}

// Note: this is a very dangerous method to call as the same identity in the past might have
// referred to a completely different node id!
async fn mixnode_identity_to_current_node_id(
    storage: &ValidatorApiStorage,
    identity: &str,
) -> Result<NodeId, ErrorResponse> {
    storage
        .mix_identity_to_latest_mix_id(identity)
        .await
        .map_err(|err| ErrorResponse::new(err.to_string(), Status::NotFound))?
        .ok_or_else(|| ErrorResponse::new("no mixnode with provided identity", Status::NotFound))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/report")]
pub(crate) async fn mixnode_report_by_identity(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<Deprecated<MixnodeStatusReport>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    Ok(Json(_mixnode_report(storage, mix_id).await?.deprecate()))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/history")]
pub(crate) async fn mixnode_uptime_history_by_identity(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<Deprecated<MixnodeUptimeHistory>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    Ok(Json(
        _mixnode_uptime_history(storage, mix_id).await?.deprecate(),
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count_by_identity(
    storage: &State<ValidatorApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Result<Json<Deprecated<MixnodeCoreStatusResponse>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    Ok(Json(
        _mixnode_core_status_count(storage, mix_id, since)
            .await?
            .deprecate(),
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/status")]
pub(crate) async fn get_mixnode_status_by_identity(
    storage: &State<ValidatorApiStorage>,
    cache: &State<ValidatorCache>,
    identity: &str,
) -> Json<Deprecated<MixnodeStatusResponse>> {
    match mixnode_identity_to_current_node_id(storage, identity).await {
        Ok(mix_id) => Json(_get_mixnode_status(cache, mix_id).await.deprecate()),
        Err(_) => Json(
            MixnodeStatusResponse {
                status: MixnodeStatus::NotFound,
            }
            .deprecate(),
        ),
    }
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<DeprecatedRewardEstimationResponse>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    let new_estimation = _get_mixnode_reward_estimation(cache, mix_id).await?;

    Ok(Json(new_estimation.into()))
}

#[openapi(tag = "status")]
#[post(
    "/mixnode/deprecated/<identity>/compute-reward-estimation",
    data = "<user_reward_param>"
)]
pub(crate) async fn compute_mixnode_reward_estimation_by_identity(
    user_reward_param: Json<DeprecatedComputeRewardEstParam>,
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<DeprecatedRewardEstimationResponse>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;
    let estimation =
        _compute_mixnode_reward_estimation(user_reward_param.into_inner().into(), cache, mix_id)
            .await?;

    Ok(Json(estimation.into()))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<Deprecated<StakeSaturationResponse>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;

    Ok(Json(
        _get_mixnode_stake_saturation(cache, mix_id)
            .await?
            .deprecate(),
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<Deprecated<InclusionProbabilityResponse>>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;

    Ok(Json(
        _get_mixnode_inclusion_probability(cache, mix_id)
            .await?
            .deprecate(),
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/deprecated/<identity>/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptime_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
    identity: &str,
) -> Result<Json<DeprecatedUptimeResponse>, ErrorResponse> {
    let mix_id = mixnode_identity_to_current_node_id(storage, identity).await?;

    let new_response = _get_mixnode_avg_uptime(cache, storage, mix_id).await?;

    Ok(Json(DeprecatedUptimeResponse {
        identity: identity.into(),
        avg_uptime: new_response.avg_uptime,
        deprecated: true,
    }))
}

// DEPRECATED: the uptime is available as part of the `/mixnodes/detailed` endpoint
#[openapi(tag = "status")]
#[get("/mixnodes/deprecated/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptimes_by_identity(
    cache: &State<ValidatorCache>,
    storage: &State<ValidatorApiStorage>,
) -> Result<Json<Vec<DeprecatedUptimeResponse>>, ErrorResponse> {
    let mixnodes = cache.mixnodes().await;

    let mut response = Vec::new();
    for mixnode in mixnodes {
        let new_response = _get_mixnode_avg_uptime(cache, storage, mixnode.mix_id()).await?;

        response.push(DeprecatedUptimeResponse {
            identity: mixnode.bond_information.identity().into(),
            avg_uptime: new_response.avg_uptime,
            deprecated: true,
        })
    }

    Ok(Json(response))
}
