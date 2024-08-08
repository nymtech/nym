// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::helpers::{
    _compute_mixnode_reward_estimation, _gateway_core_status_count, _gateway_report,
    _gateway_uptime_history, _get_active_set_detailed, _get_gateway_avg_uptime,
    _get_gateways_detailed, _get_gateways_detailed_unfiltered, _get_mixnode_avg_uptime,
    _get_mixnode_inclusion_probabilities, _get_mixnode_inclusion_probability,
    _get_mixnode_reward_estimation, _get_mixnode_stake_saturation, _get_mixnode_status,
    _get_mixnodes_detailed, _get_mixnodes_detailed_unfiltered, _get_rewarded_set_detailed,
    _mixnode_core_status_count, _mixnode_report, _mixnode_uptime_history,
};
use crate::node_status_api::models::AxumResult;
use crate::v2::AxumAppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use axum::Router;
use nym_api_requests::models::{
    AllInclusionProbabilitiesResponse, ComputeRewardEstParam, GatewayBondAnnotated,
    GatewayCoreStatusResponse, GatewayStatusReportResponse, GatewayUptimeHistoryResponse,
    GatewayUptimeResponse, InclusionProbabilityResponse, MixNodeBondAnnotated,
    MixnodeCoreStatusResponse, MixnodeStatusReportResponse, MixnodeStatusResponse,
    MixnodeUptimeHistoryResponse, RewardEstimationResponse, StakeSaturationResponse,
    UptimeResponse,
};
use nym_mixnet_contract_common::MixId;

pub(crate) fn node_status_routes(network_monitor: bool) -> Router<AxumAppState> {
    // in the minimal variant we would not have access to endpoints relying on existence
    // of the network monitor and the associated storage
    let without_network_monitor = Router::new().nest(
        "/v1/gateway/:identity",
        Router::new()
            .nest(
                "/mixnode/:mix_id",
                Router::new()
                    .route("/status", axum::routing::get(get_mixnode_status))
                    .route(
                        "/stake-saturation",
                        axum::routing::get(get_mixnode_stake_saturation),
                    )
                    .route(
                        "/inclusion-probability",
                        axum::routing::get(get_mixnode_inclusion_probability),
                    ),
            )
            .nest(
                "/mixnodes",
                Router::new()
                    .route(
                        "/inclusion-probability",
                        axum::routing::get(get_mixnode_inclusion_probabilities),
                    )
                    .route("/detailed", axum::routing::get(get_mixnodes_detailed))
                    .route(
                        "/rewarded/detailed",
                        axum::routing::get(get_rewarded_set_detailed),
                    )
                    .route(
                        "/active/detailed",
                        axum::routing::get(get_active_set_detailed),
                    ),
            ),
    );

    if network_monitor {
        let with_network_monitor = Router::new().nest(
            "/v1/gateway/:identity",
            Router::new()
                .route("/report", axum::routing::get(gateway_report))
                .route("/history", axum::routing::get(gateway_uptime_history))
                .route(
                    "/core-status-count",
                    axum::routing::get(gateway_core_status_count),
                )
                .route("/avg_uptime", axum::routing::get(get_gateway_avg_uptime))
                .nest(
                    "/mixnode/:mix_id",
                    Router::new()
                        .route("/report", axum::routing::get(mixnode_report))
                        .route("/history", axum::routing::get(mixnode_uptime_history))
                        .route(
                            "/core-status-count",
                            axum::routing::get(mixnode_core_status_count),
                        )
                        .route(
                            "/reward-estimation",
                            axum::routing::get(get_mixnode_reward_estimation),
                        )
                        .route(
                            "/compute-reward-estimation",
                            axum::routing::post(compute_mixnode_reward_estimation),
                        )
                        .route("/avg_uptime", axum::routing::get(get_mixnode_avg_uptime)),
                )
                .nest(
                    "/mixnodes",
                    Router::new()
                        .route(
                            "/detailed-unfiltered",
                            axum::routing::get(get_mixnodes_detailed_unfiltered),
                        )
                        .route(
                            "/unstable/:mix_id/test-results",
                            axum::routing::get(unstable::mixnode_test_results),
                        ),
                )
                .nest(
                    "/gateways",
                    Router::new()
                        .route("/detailed", axum::routing::get(get_gateways_detailed))
                        .route(
                            "/detailed-unfiltered",
                            axum::routing::get(get_gateways_detailed_unfiltered),
                        )
                        .route(
                            "/unstable/:gateway_identity/test-results",
                            axum::routing::get(unstable::gateway_test_results),
                        ),
                ),
        );

        with_network_monitor.merge(without_network_monitor)
    } else {
        without_network_monitor
    }
}

pub(crate) async fn gateway_report(
    Path(identity): Path<String>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayStatusReportResponse>> {
    Ok(Json(
        _gateway_report(state.node_status_cache(), &identity).await?,
    ))
}

pub(crate) async fn gateway_uptime_history(
    Path(identity): Path<String>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayUptimeHistoryResponse>> {
    Ok(Json(
        _gateway_uptime_history(state.storage(), &identity).await?,
    ))
}

pub(crate) async fn gateway_core_status_count(
    Path(identity): Path<String>,
    Query(since): Query<Option<i64>>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayCoreStatusResponse>> {
    Ok(Json(
        _gateway_core_status_count(state.storage(), &identity, since).await?,
    ))
}

pub(crate) async fn mixnode_report(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<MixnodeStatusReportResponse>> {
    Ok(Json(
        _mixnode_report(state.node_status_cache(), mix_id).await?,
    ))
}

pub(crate) async fn mixnode_uptime_history(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<MixnodeUptimeHistoryResponse>> {
    Ok(Json(
        _mixnode_uptime_history(state.storage(), mix_id).await?,
    ))
}

pub(crate) async fn mixnode_core_status_count(
    Path(mix_id): Path<MixId>,
    Query(since): Query<Option<i64>>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<MixnodeCoreStatusResponse>> {
    Ok(Json(
        _mixnode_core_status_count(state.storage(), mix_id, since).await?,
    ))
}

pub(crate) async fn get_mixnode_status(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> Json<MixnodeStatusResponse> {
    Json(_get_mixnode_status(state.nym_contract_cache(), mix_id).await)
}

pub(crate) async fn get_mixnode_reward_estimation(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<RewardEstimationResponse>> {
    Ok(Json(
        _get_mixnode_reward_estimation(
            state.node_status_cache(),
            state.nym_contract_cache(),
            mix_id,
        )
        .await?,
    ))
}

pub(crate) async fn compute_mixnode_reward_estimation(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
    Json(user_reward_param): Json<ComputeRewardEstParam>,
) -> AxumResult<Json<RewardEstimationResponse>> {
    Ok(Json(
        _compute_mixnode_reward_estimation(
            &user_reward_param,
            state.node_status_cache(),
            state.nym_contract_cache(),
            mix_id,
        )
        .await?,
    ))
}

pub(crate) async fn get_mixnode_stake_saturation(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<StakeSaturationResponse>> {
    Ok(Json(
        _get_mixnode_stake_saturation(
            state.node_status_cache(),
            state.nym_contract_cache(),
            mix_id,
        )
        .await?,
    ))
}

pub(crate) async fn get_mixnode_inclusion_probability(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<InclusionProbabilityResponse>> {
    Ok(Json(
        _get_mixnode_inclusion_probability(state.node_status_cache(), mix_id).await?,
    ))
}

pub(crate) async fn get_mixnode_avg_uptime(
    Path(mix_id): Path<MixId>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<UptimeResponse>> {
    Ok(Json(
        _get_mixnode_avg_uptime(state.node_status_cache(), mix_id).await?,
    ))
}

pub(crate) async fn get_gateway_avg_uptime(
    Path(identity): Path<String>,
    State(state): State<AxumAppState>,
) -> AxumResult<Json<GatewayUptimeResponse>> {
    Ok(Json(
        _get_gateway_avg_uptime(state.node_status_cache(), &identity).await?,
    ))
}

pub(crate) async fn get_mixnode_inclusion_probabilities(
    State(state): State<AxumAppState>,
) -> AxumResult<Json<AllInclusionProbabilitiesResponse>> {
    Ok(Json(
        _get_mixnode_inclusion_probabilities(state.node_status_cache()).await?,
    ))
}

pub async fn get_mixnodes_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed(state.node_status_cache()).await)
}

pub async fn get_mixnodes_detailed_unfiltered(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_mixnodes_detailed_unfiltered(state.node_status_cache()).await)
}

pub async fn get_rewarded_set_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_rewarded_set_detailed(state.node_status_cache()).await)
}

pub async fn get_active_set_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<MixNodeBondAnnotated>> {
    Json(_get_active_set_detailed(state.node_status_cache()).await)
}

pub async fn get_gateways_detailed(
    State(state): State<AxumAppState>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed(state.node_status_cache()).await)
}

pub async fn get_gateways_detailed_unfiltered(
    State(state): State<AxumAppState>,
) -> Json<Vec<GatewayBondAnnotated>> {
    Json(_get_gateways_detailed_unfiltered(state.node_status_cache()).await)
}

pub mod unstable {
    use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
    use crate::support::http::helpers::PaginationRequest;
    use crate::support::storage::NymApiStorage;
    use crate::v2::AxumAppState;
    use axum::extract::{Path, Query, State};
    use axum::Json;
    use nym_api_requests::models::{
        GatewayTestResultResponse, MixnodeTestResultResponse, PartialTestResult, TestNode,
        TestRoute,
    };
    use nym_api_requests::pagination::Pagination;
    use nym_mixnet_contract_common::MixId;
    use std::cmp::min;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    pub type DbId = i64;

    // a simply in-memory cache of node details
    #[derive(Debug, Clone, Default)]
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

    #[derive(Debug, Clone, Default)]
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
        info_cache: &NodeInfoCache,
        storage: &NymApiStorage,
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

    pub async fn mixnode_test_results(
        Path(mix_id): Path<MixId>,
        Query(pagination): Query<PaginationRequest>,
        State(state): State<AxumAppState>,
    ) -> AxumResult<Json<MixnodeTestResultResponse>> {
        let page = pagination.page.unwrap_or_default();
        let per_page = min(
            pagination
                .per_page
                .unwrap_or(DEFAULT_TEST_RESULTS_PAGE_SIZE),
            MAX_TEST_RESULTS_PAGE_SIZE,
        );

        match _mixnode_test_results(
            mix_id,
            page,
            per_page,
            state.node_info_cache(),
            state.storage(),
        )
        .await
        {
            Ok(res) => Ok(Json(res)),
            Err(err) => Err(AxumErrorResponse::internal_msg(format!(
                "failed to retrieve mixnode test results for node {mix_id}: {err}"
            ))),
        }
    }

    async fn _gateway_test_results(
        gateway_identity: &str,
        page: u32,
        per_page: u32,
        info_cache: &NodeInfoCache,
        storage: &NymApiStorage,
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

    pub async fn gateway_test_results(
        Path(gateway_identity): Path<String>,
        Query(pagination): Query<PaginationRequest>,
        State(state): State<AxumAppState>,
    ) -> AxumResult<Json<GatewayTestResultResponse>> {
        let page = pagination.page.unwrap_or_default();
        let per_page = min(
            pagination
                .per_page
                .unwrap_or(DEFAULT_TEST_RESULTS_PAGE_SIZE),
            MAX_TEST_RESULTS_PAGE_SIZE,
        );

        match _gateway_test_results(
            &gateway_identity,
            page,
            per_page,
            state.node_info_cache(),
            state.storage(),
        )
        .await
        {
            Ok(res) => Ok(Json(res)),
            Err(err) => Err(AxumErrorResponse::internal_msg(format!(
                "failed to retrieve mixnode test results for gateway {gateway_identity}: {err}"
            ))),
        }
    }
}
