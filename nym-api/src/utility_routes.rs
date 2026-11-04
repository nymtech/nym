// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::mixnet_contract_cache::MixnetContractCacheState;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::Router;
use nym_api_requests::models::utility::{
    MixnetContractCacheTimestampResponse, NodeStatusCacheTimestampResponse,
    RefreshMixnetContractCacheRequestBody, RefreshMixnetContractCacheResponse,
};
use nym_http_api_common::{FormattedResponse, OutputParams};
use std::time::Duration;
use time::OffsetDateTime;

#[allow(deprecated)]
pub(crate) fn utility_routes() -> Router<AppState> {
    Router::new()
        .route("/refresh-mixnet-cache", post(refresh_mixnet_cache))
        .route("/mixnet-cache-timestamp", get(mixnet_cache_timestamp))
        .route(
            "/node-annotations-cache-timestamp",
            get(node_annotations_cache_timestamp),
        )
}

/// Allow to request to refresh the cache of all mixnet nodes on the network.
/// Note that this endpoint enforces high global rate limiting and realistically
/// should not be used outside very special scenarios.
#[utoipa::path(
    tag = "Utility",
    post,
    request_body = RefreshMixnetContractCacheRequestBody,
    path = "/refresh-mixnet-cache",
    context_path = "/v1/utility",
    responses(
        (status = 200, content(
            (RefreshMixnetContractCacheResponse = "application/json"),
            (RefreshMixnetContractCacheResponse = "application/yaml"),
            (RefreshMixnetContractCacheResponse = "application/bincode")
        ))
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
async fn refresh_mixnet_cache(
    Query(output): Query<OutputParams>,
    State(cache): State<MixnetContractCacheState>,
) -> AxumResult<FormattedResponse<RefreshMixnetContractCacheResponse>> {
    let output = output.get_output();
    let now = OffsetDateTime::now_utc();

    // max 1 refresh every 5min (TODO: make it configurable)
    let cutoff = now - Duration::from_secs(5 * 60);
    let last = cache.refresh_handle.last_requested();
    if last > cutoff {
        return Err(AxumErrorResponse::too_many(
            "already refreshed contract cache in the last 5 minutes",
        ));
    }
    cache.refresh_handle.request_refresh(now);

    Ok(output.to_response(RefreshMixnetContractCacheResponse { success: true }))
}

/// Return information on when the mixnet cache has last been refreshed.
#[utoipa::path(
    tag = "Utility",
    get,
    path = "/mixnet-cache-timestamp",
    context_path = "/v1/utility",
    responses(
        (status = 200, content(
            (MixnetContractCacheTimestampResponse = "application/json"),
            (MixnetContractCacheTimestampResponse = "application/yaml"),
            (MixnetContractCacheTimestampResponse = "application/bincode")
        ))
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
async fn mixnet_cache_timestamp(
    Query(output): Query<OutputParams>,
    State(cache): State<MixnetContractCache>,
) -> FormattedResponse<MixnetContractCacheTimestampResponse> {
    let output = output.get_output();
    let timestamp = cache.cache_timestamp().await;
    output.to_response(MixnetContractCacheTimestampResponse { timestamp })
}

/// Deprecated: there is no annotation cache any more; this answers the block time of the latest
/// settled performance snapshot, which every node's published performance is read from.
#[utoipa::path(
    tag = "Utility",
    get,
    path = "/node-annotations-cache-timestamp",
    context_path = "/v1/utility",
    responses(
        (status = 200, content(
            (NodeStatusCacheTimestampResponse = "application/json"),
            (NodeStatusCacheTimestampResponse = "application/yaml"),
            (NodeStatusCacheTimestampResponse = "application/bincode")
        ))
    ),
    params(OutputParams),
    security(
        ("auth_token" = [])
    )
)]
#[deprecated]
async fn node_annotations_cache_timestamp(
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> FormattedResponse<NodeStatusCacheTimestampResponse> {
    let output = output.get_output();
    let timestamp = match state.contract_snapshots.latest_entry().await {
        Some(snapshot) => snapshot.performance_auxiliary.block_time(),
        None => OffsetDateTime::UNIX_EPOCH,
    };
    output.to_response(NodeStatusCacheTimestampResponse { timestamp })
}
