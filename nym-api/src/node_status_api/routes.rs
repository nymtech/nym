// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::{
    AllInclusionProbabilitiesResponse, ComputeRewardEstParam, GatewayBondAnnotated,
    GatewayCoreStatusResponse, GatewayStatusReportResponse, GatewayUptimeHistoryResponse,
    GatewayUptimeResponse, InclusionProbabilityResponse, MixNodeBondAnnotated,
    MixnodeCoreStatusResponse, MixnodeStatusReportResponse, MixnodeStatusResponse,
    MixnodeUptimeHistoryResponse, RewardEstimationResponse, StakeSaturationResponse,
    UptimeResponse,
};
use nym_mixnet_contract_common::MixId;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;

use super::helpers::_get_gateways_detailed;
use super::NodeStatusCache;
use crate::node_status_api::helpers::{
    _compute_mixnode_reward_estimation, _gateway_core_status_count, _gateway_report,
    _gateway_uptime_history, _get_active_set_detailed, _get_gateway_avg_uptime,
    _get_gateways_detailed_unfiltered, _get_mixnode_avg_uptime,
    _get_mixnode_inclusion_probabilities, _get_mixnode_inclusion_probability,
    _get_mixnode_reward_estimation, _get_mixnode_stake_saturation, _get_mixnode_status,
    _get_mixnodes_detailed, _get_mixnodes_detailed_unfiltered, _get_rewarded_set_detailed,
    _mixnode_core_status_count, _mixnode_report, _mixnode_uptime_history,
};
use crate::node_status_api::models::ErrorResponse;
use crate::storage::NymApiStorage;
use crate::NymContractCache;

#[openapi(tag = "status")]
#[get("/gateway/<identity>/report")]
pub(crate) async fn gateway_report(
    cache: &State<NodeStatusCache>,
    identity: &str,
) -> Result<Json<GatewayStatusReportResponse>, ErrorResponse> {
    Ok(Json(_gateway_report(cache, identity).await?))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/history")]
pub(crate) async fn gateway_uptime_history(
    storage: &State<NymApiStorage>,
    identity: &str,
) -> Result<Json<GatewayUptimeHistoryResponse>, ErrorResponse> {
    Ok(Json(_gateway_uptime_history(storage, identity).await?))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/core-status-count?<since>")]
pub(crate) async fn gateway_core_status_count(
    storage: &State<NymApiStorage>,
    identity: &str,
    since: Option<i64>,
) -> Result<Json<GatewayCoreStatusResponse>, ErrorResponse> {
    Ok(Json(
        _gateway_core_status_count(storage, identity, since).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/report")]
pub(crate) async fn mixnode_report(
    cache: &State<NodeStatusCache>,
    mix_id: MixId,
) -> Result<Json<MixnodeStatusReportResponse>, ErrorResponse> {
    Ok(Json(_mixnode_report(cache, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/history")]
pub(crate) async fn mixnode_uptime_history(
    storage: &State<NymApiStorage>,
    mix_id: MixId,
) -> Result<Json<MixnodeUptimeHistoryResponse>, ErrorResponse> {
    Ok(Json(_mixnode_uptime_history(storage, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/core-status-count?<since>")]
pub(crate) async fn mixnode_core_status_count(
    storage: &State<NymApiStorage>,
    mix_id: MixId,
    since: Option<i64>,
) -> Result<Json<MixnodeCoreStatusResponse>, ErrorResponse> {
    Ok(Json(
        _mixnode_core_status_count(storage, mix_id, since).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/status")]
pub(crate) async fn get_mixnode_status(
    cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Json<MixnodeStatusResponse> {
    Json(_get_mixnode_status(cache, mix_id).await)
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/reward-estimation")]
pub(crate) async fn get_mixnode_reward_estimation(
    cache: &State<NodeStatusCache>,
    validator_cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    Ok(Json(
        _get_mixnode_reward_estimation(cache, validator_cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[post(
    "/mixnode/<mix_id>/compute-reward-estimation",
    data = "<user_reward_param>"
)]
pub(crate) async fn compute_mixnode_reward_estimation(
    user_reward_param: Json<ComputeRewardEstParam>,
    cache: &State<NodeStatusCache>,
    validator_cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Result<Json<RewardEstimationResponse>, ErrorResponse> {
    Ok(Json(
        _compute_mixnode_reward_estimation(
            user_reward_param.into_inner(),
            cache,
            validator_cache,
            mix_id,
        )
        .await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/stake-saturation")]
pub(crate) async fn get_mixnode_stake_saturation(
    cache: &State<NodeStatusCache>,
    validator_cache: &State<NymContractCache>,
    mix_id: MixId,
) -> Result<Json<StakeSaturationResponse>, ErrorResponse> {
    Ok(Json(
        _get_mixnode_stake_saturation(cache, validator_cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/inclusion-probability")]
pub(crate) async fn get_mixnode_inclusion_probability(
    cache: &State<NodeStatusCache>,
    mix_id: MixId,
) -> Result<Json<InclusionProbabilityResponse>, ErrorResponse> {
    Ok(Json(
        _get_mixnode_inclusion_probability(cache, mix_id).await?,
    ))
}

#[openapi(tag = "status")]
#[get("/mixnode/<mix_id>/avg_uptime")]
pub(crate) async fn get_mixnode_avg_uptime(
    cache: &State<NodeStatusCache>,
    mix_id: MixId,
) -> Result<Json<UptimeResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_avg_uptime(cache, mix_id).await?))
}

#[openapi(tag = "status")]
#[get("/gateway/<identity>/avg_uptime")]
pub(crate) async fn get_gateway_avg_uptime(
    cache: &State<NodeStatusCache>,
    identity: &str,
) -> Result<Json<GatewayUptimeResponse>, ErrorResponse> {
    Ok(Json(_get_gateway_avg_uptime(cache, identity).await?))
}

#[openapi(tag = "status")]
#[get("/mixnodes/inclusion_probability")]
pub(crate) async fn get_mixnode_inclusion_probabilities(
    cache: &State<NodeStatusCache>,
) -> Result<Json<AllInclusionProbabilitiesResponse>, ErrorResponse> {
    Ok(Json(_get_mixnode_inclusion_probabilities(cache).await?))
}

#[openapi(tag = "status")]
#[get("/mixnodes/detailed")]
pub async fn get_mixnodes_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/mixnodes/detailed-unfiltered")]
pub async fn get_mixnodes_detailed_unfiltered(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed_unfiltered(cache).await)
}

#[openapi(tag = "status")]
#[get("/mixnodes/rewarded/detailed")]
pub async fn get_rewarded_set_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_rewarded_set_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/mixnodes/active/detailed")]
pub async fn get_active_set_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_active_set_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/gateways/detailed")]
pub async fn get_gateways_detailed(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed(cache).await)
}

#[openapi(tag = "status")]
#[get("/gateways/detailed-unfiltered")]
pub async fn get_gateways_detailed_unfiltered(
    cache: &State<NodeStatusCache>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed_unfiltered(cache).await)
}

pub mod unstable {
    use crate::node_status_api::models::ErrorResponse;
    use crate::support::http::helpers::PaginationRequest;
    use crate::support::storage::NymApiStorage;
    use nym_api_requests::models::{
        GatewayTestResultResponse, MixnodeTestResultResponse, PartialTestResult, TestNode,
        TestRoute,
    };
    use nym_api_requests::pagination::Pagination;
    use nym_mixnet_contract_common::MixId;
    use rocket::http::Status;
    use rocket::serde::json::Json;
    use rocket::State;
    use rocket_okapi::openapi;
    use std::cmp::min;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    pub type DbId = i64;

    // a simply in-memory cache of node details
    #[derive(Debug, Default)]
    pub struct NodeInfoCache {
        inner: Arc<RwLock<NodeInfoCacheInner>>,
    }

    impl NodeInfoCache {
        async fn get_mix_node_details(&self, db_id: DbId, storage: &NymApiStorage) -> TestNode {
            {
                let read_guard = self.inner.read().await;
                if let Some(cached) = read_guard.mixnodes.get(&db_id) {
                    trace!("cache hit for mixnode {db_id}");
                    return cached.clone();
                }
            }
            trace!("cache miss for mixnode {db_id}");

            let mut write_guard = self.inner.write().await;
            // double-check the cache in case somebody already updated it while we were waiting for the lock
            if let Some(cached) = write_guard.mixnodes.get(&db_id) {
                return cached.clone();
            }

            let details = match storage.get_mixnode_details_by_db_id(db_id).await {
                Ok(Some(details)) => details.into(),
                Ok(None) => {
                    error!("somebody has been messing with the database! details for mixnode with database id {db_id} have been removed!");
                    TestNode::default()
                }
                Err(err) => {
                    // don't insert into the cache in case another request is successful
                    error!("failed to retrieve details for mixnode {db_id}: {err}");
                    return TestNode::default();
                }
            };

            write_guard.mixnodes.insert(db_id, details.clone());
            details
        }

        async fn get_gateway_details(&self, db_id: DbId, storage: &NymApiStorage) -> TestNode {
            {
                let read_guard = self.inner.read().await;
                if let Some(cached) = read_guard.gateways.get(&db_id) {
                    trace!("cache hit for gateway {db_id}");
                    return cached.clone();
                }
            }
            trace!("cache miss for gateway {db_id}");

            let mut write_guard = self.inner.write().await;
            // double-check the cache in case somebody already updated it while we were waiting for the lock
            if let Some(cached) = write_guard.gateways.get(&db_id) {
                return cached.clone();
            }

            let details = match storage.get_gateway_details_by_db_id(db_id).await {
                Ok(Some(details)) => details.into(),
                Ok(None) => {
                    error!("somebody has been messing with the database! details for gateway with database id {db_id} have been removed!");
                    TestNode::default()
                }
                Err(err) => {
                    // don't insert into the cache in case another request is successful
                    error!("failed to retrieve details for gateway {db_id}: {err}");
                    return TestNode::default();
                }
            };

            write_guard.gateways.insert(db_id, details.clone());
            details
        }
    }

    #[derive(Debug, Default)]
    struct NodeInfoCacheInner {
        mixnodes: HashMap<DbId, TestNode>,
        gateways: HashMap<DbId, TestNode>,
    }

    const MAX_TEST_RESULTS_PAGE_SIZE: u32 = 100;
    const DEFAULT_TEST_RESULTS_PAGE_SIZE: u32 = 50;

    async fn _mixnode_test_results(
        mix_id: MixId,
        page: u32,
        per_page: u32,
        info_cache: &State<NodeInfoCache>,
        storage: &State<NymApiStorage>,
    ) -> anyhow::Result<MixnodeTestResultResponse> {
        // convert to db offset
        // we're paging from page 0 like civilised people,
        // so we have to skip (page * per_page) results
        let offset = page * per_page;
        let limit = per_page;

        let raw_results = storage
            .get_mixnode_detailed_statuses(mix_id, limit, offset)
            .await?;
        let total = match raw_results.first() {
            None => 0,
            Some(r) => storage.get_mixnode_detailed_statuses_count(r.db_id).await?,
        };

        let mut partial_results = Vec::new();
        for result in raw_results {
            let gateway = info_cache
                .get_gateway_details(result.gateway_id, storage)
                .await;
            let layer1 = info_cache
                .get_mix_node_details(result.layer1_mix_id, storage)
                .await;
            let layer2 = info_cache
                .get_mix_node_details(result.layer2_mix_id, storage)
                .await;
            let layer3 = info_cache
                .get_mix_node_details(result.layer3_mix_id, storage)
                .await;

            partial_results.push(PartialTestResult {
                monitor_run_id: result.monitor_run_id,
                timestamp: result.timestamp,
                overall_reliability_for_all_routes_in_monitor_run: result.reliability,
                test_routes: TestRoute {
                    gateway,
                    layer1,
                    layer2,
                    layer3,
                },
            })
        }

        Ok(MixnodeTestResultResponse {
            pagination: Pagination {
                total,
                page,
                size: partial_results.len(),
            },
            data: partial_results,
        })
    }

    #[openapi(tag = "UNSTABLE - DO **NOT** USE")]
    #[get("/mixnodes/unstable/<mix_id>/test-results?<pagination..>")]
    pub async fn mixnode_test_results(
        mix_id: MixId,
        pagination: PaginationRequest,
        info_cache: &State<NodeInfoCache>,
        storage: &State<NymApiStorage>,
    ) -> Result<Json<MixnodeTestResultResponse>, ErrorResponse> {
        let page = pagination.page.unwrap_or_default();
        let per_page = min(
            pagination
                .per_page
                .unwrap_or(DEFAULT_TEST_RESULTS_PAGE_SIZE),
            MAX_TEST_RESULTS_PAGE_SIZE,
        );

        match _mixnode_test_results(mix_id, page, per_page, info_cache, storage).await {
            Ok(res) => Ok(Json(res)),
            Err(err) => Err(ErrorResponse::new(
                format!("failed to retrieve mixnode test results for node {mix_id}: {err}"),
                Status::InternalServerError,
            )),
        }
    }

    async fn _gateway_test_results(
        gateway_identity: &str,
        page: u32,
        per_page: u32,
        info_cache: &State<NodeInfoCache>,
        storage: &State<NymApiStorage>,
    ) -> anyhow::Result<GatewayTestResultResponse> {
        // convert to db offset
        // we're paging from page 0 like civilised people,
        // so we have to skip (page * per_page) results
        let offset = page * per_page;
        let limit = per_page;

        let raw_results = storage
            .get_gateway_detailed_statuses(gateway_identity, limit, offset)
            .await?;
        let total = match raw_results.first() {
            None => 0,
            Some(r) => storage.get_gateway_detailed_statuses_count(r.db_id).await?,
        };

        let mut partial_results = Vec::new();
        for result in raw_results {
            let gateway = info_cache
                .get_gateway_details(result.gateway_id, storage)
                .await;
            let layer1 = info_cache
                .get_mix_node_details(result.layer1_mix_id, storage)
                .await;
            let layer2 = info_cache
                .get_mix_node_details(result.layer2_mix_id, storage)
                .await;
            let layer3 = info_cache
                .get_mix_node_details(result.layer3_mix_id, storage)
                .await;

            partial_results.push(PartialTestResult {
                monitor_run_id: result.monitor_run_id,
                timestamp: result.timestamp,
                overall_reliability_for_all_routes_in_monitor_run: result.reliability,
                test_routes: TestRoute {
                    gateway,
                    layer1,
                    layer2,
                    layer3,
                },
            })
        }

        Ok(GatewayTestResultResponse {
            pagination: Pagination {
                total,
                page,
                size: partial_results.len(),
            },
            data: partial_results,
        })
    }

    #[openapi(tag = "UNSTABLE - DO **NOT** USE")]
    #[get("/gateways/unstable/<gateway_identity>/test-results?<pagination..>")]
    pub async fn gateway_test_results(
        gateway_identity: &str,
        pagination: PaginationRequest,
        info_cache: &State<NodeInfoCache>,
        storage: &State<NymApiStorage>,
    ) -> Result<Json<GatewayTestResultResponse>, ErrorResponse> {
        let page = pagination.page.unwrap_or_default();
        let per_page = min(
            pagination
                .per_page
                .unwrap_or(DEFAULT_TEST_RESULTS_PAGE_SIZE),
            MAX_TEST_RESULTS_PAGE_SIZE,
        );

        match _gateway_test_results(gateway_identity, page, per_page, info_cache, storage).await {
            Ok(res) => Ok(Json(res)),
            Err(err) => Err(ErrorResponse::new(
                format!(
                    "failed to retrieve mixnode test results for gateway {gateway_identity}: {err}"
                ),
                Status::InternalServerError,
            )),
        }
    }
}
