// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::handlers::MixIdParam;
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::helpers::PaginationRequest;
use crate::support::http::state::AppState;
use crate::support::storage::NymApiStorage;
use axum::extract::{Path, Query, State};
use axum::Json;
use nym_api_requests::models::{
    GatewayTestResultResponse, MixnodeTestResultResponse, PartialTestResult, TestNode, TestRoute,
};
use nym_api_requests::pagination::Pagination;
use nym_mixnet_contract_common::NodeId;
use std::cmp::min;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, trace};

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
    mix_id: NodeId,
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

#[utoipa::path(
    tag = "UNSTABLE - DO **NOT** USE",
    get,
    params(
        MixIdParam, PaginationRequest
    ),
    path = "/v1/status/mixnodes/unstable/{mix_id}/test-results",
    responses(
        (status = 200, body = MixnodeTestResultResponse)
    )
)]
pub async fn mixnode_test_results(
    Path(mix_id): Path<NodeId>,
    Query(pagination): Query<PaginationRequest>,
    State(state): State<AppState>,
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

#[utoipa::path(
    tag = "UNSTABLE - DO **NOT** USE",
    get,
    params(
        PaginationRequest
    ),
    path = "/v1/status/gateways/unstable/{identity}/test-results",
    responses(
        (status = 200, body = GatewayTestResultResponse)
    )
)]
pub async fn gateway_test_results(
    Path(gateway_identity): Path<String>,
    Query(pagination): Query<PaginationRequest>,
    State(state): State<AppState>,
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
