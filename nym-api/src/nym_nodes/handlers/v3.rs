// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{ApiResult, AxumErrorResponse};
use crate::support::http::state::AppState;
use crate::support::storage::models::NymNodeStressTestingResult;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_api_requests::models::network_monitor::KnownNetworkMonitorResponse;
use nym_api_requests::models::StressTestBatchSubmission;
use nym_crypto::asymmetric::ed25519;
use std::time::Duration;
use tracing::error;

#[utoipa::path(
    tag = "Nym Nodes",
    post,
    path = "/stress-testing/batch-submit",
    context_path = "/v3/nym-nodes",
    responses(
        (status = 200, description = "the submitted batch has been accepted and stored"),
        (status = 400, description = "the submitted request is stale or replayed"),
        (status = 401, description = "the submitted request was unauthorised or failed integrity check"),
    ),
)]
async fn batch_submit_stress_testing_results(
    State(state): State<AppState>,
    Json(body): Json<StressTestBatchSubmission>,
) -> ApiResult<()> {
    // 1. check if the request is not stale
    // TODO: make the timeout configurable. currently we have an issue of no easy way of
    // propagating config values, but hardcoding it to 30s is fine for now
    if body.body.is_stale(Duration::from_secs(30)) {
        return Err(AxumErrorResponse::bad_request(
            "request is stale, please resubmit it with a fresh timestamp",
        ));
    }

    // 2. check if the sent public key is even in the authorised set
    if !state
        .network_monitors()
        .is_authorised(&state.nyxd_client, &body.body.signer)
        .await?
    {
        return Err(AxumErrorResponse::unauthorised(
            "the provided public key does not correspond to any known network monitor",
        ));
    }

    // 3. check if the request is not replayed (i.e. timestamp is not smaller than the latest known submission)
    let last_request = state
        .network_monitor_submissions
        .submitted(body.body.signer)
        .await;

    if body.body.timestamp <= last_request {
        return Err(AxumErrorResponse::bad_request(
            "each request must have an explicitly greater timestamp than the previous one",
        ));
    }

    // 4. verify the signature on the request
    if !body.verify_signature(&body.body.signer) {
        return Err(AxumErrorResponse::unauthorised(
            "the provided request failed integrity check",
        ));
    }

    // 5. update the latest submission timestamp
    state
        .network_monitor_submissions
        .set_submitted(body.body.signer, body.body.timestamp)
        .await;

    // 6. process received results
    let signer = body.body.signer;
    let mut mixnode_results = Vec::with_capacity(body.body.results.len());
    for result in body.body.results {
        if result.is_mixnode {
            mixnode_results.push(NymNodeStressTestingResult::from(result));
        } else {
            error!(
                %signer,
                node_id = result.node_id,
                "received a stress testing result for a non-mixnode entry which should never happen - is the nym-api outdated?"
            );
        }
    }

    state
        .storage()
        .insert_nym_node_stress_testing_results(mixnode_results)
        .await?;

    Ok(())
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    path = "/stress-testing/known-monitors/{identity_key}",
    context_path = "/v3/nym-nodes",
    params(
        ("identity_key" = String, Path, description = "base58-encoded ed25519 identity key of the queried network monitor"),
    ),
    responses(
        (status = 200, body = KnownNetworkMonitorResponse),
        (status = 400, description = "the provided identity key is not a valid base58-encoded ed25519 public key"),
    ),
)]
async fn known_network_monitor(
    Path(identity_key): Path<String>,
    State(state): State<AppState>,
) -> ApiResult<Json<KnownNetworkMonitorResponse>> {
    let identity_key = ed25519::PublicKey::from_base58_string(&identity_key)
        .map_err(|err| AxumErrorResponse::bad_request(format!("malformed identity key: {err}")))?;

    let known = state
        .network_monitors()
        .get_or_refresh(&state.nyxd_client)
        .await?;

    let authorised = known.contains(&identity_key);

    Ok(Json(KnownNetworkMonitorResponse {
        identity_key,
        authorised,
    }))
}

fn stress_testing_routes() -> Router<AppState> {
    Router::new()
        .route("/batch-submit", post(batch_submit_stress_testing_results))
        .route("/known-monitors/:identity_key", get(known_network_monitor))
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new().nest("/stress-testing", stress_testing_routes())
}
