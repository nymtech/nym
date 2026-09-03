// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{ApiResult, AxumErrorResponse};
use crate::support::http::state::AppState;
use crate::support::storage::models::NymNodeStressTestingResult;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_api_requests::models::v3::{
    KnownNetworkMonitorResponse, LivenessTestBatchSubmission, LivenessTestBatchSubmissionResponse,
    StressTestBatchSubmission, StressTestBatchSubmissionResponse,
};
use nym_crypto::asymmetric::ed25519;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{error, warn};

/// How old a submission body may be before either ingest endpoint refuses it.
///
/// Shared by both endpoints rather than written out at each: two independent literals would let
/// the windows drift apart with nothing failing to compile, and there is no reason for an
/// orchestrator to face a different deadline depending on which stream it is draining. Not a
/// config value - nobody has asked to tune it, and the delta spec fixes 30 seconds as normative -
/// but this const is the single place that would become a config read if that changes.
const SUBMISSION_STALENESS_WINDOW: Duration = Duration::from_secs(30);

/// Accept a batch of stress-test results from an authorised network monitor orchestrator.
///
/// The batch is rejected unless all of the following hold:
/// - the submission timestamp is within a short staleness window of the current time,
/// - the signer's key is currently registered in the network-monitors contract,
/// - the submission timestamp is strictly greater than the signer's previous accepted submission
///   (timestamp-based replay protection, so orchestrators don't need to keep a nonce counter),
/// - the signature on the body verifies against the signer's key.
///
/// Individual result entries that fail per-entry validation (non-mixnode role, performance score
/// outside `[0.0, 1.0]`) are logged as errors and dropped, but do not fail the batch.
///
/// The response reports how many results were stored, deduplicated against an already-stored
/// measurement, and dropped by validation, so that a submitter can tell an accepted-and-stored
/// batch from an accepted-but-discarded one.
#[utoipa::path(
    tag = "Nym Nodes",
    post,
    path = "/stress-testing/batch-submit",
    context_path = "/v3/nym-nodes",
    responses(
        (status = 200, body = StressTestBatchSubmissionResponse, description = "the submitted batch has been accepted, with a per-result breakdown of what was stored"),
        (status = 400, description = "the submitted request is stale or replayed"),
        (status = 401, description = "the submitted request was unauthorised or failed integrity check"),
    ),
)]
async fn batch_submit_stress_testing_results(
    State(state): State<AppState>,
    Json(body): Json<StressTestBatchSubmission>,
) -> ApiResult<Json<StressTestBatchSubmissionResponse>> {
    // 1. check if the request is not stale
    if body.body.is_stale(SUBMISSION_STALENESS_WINDOW) {
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

    // 3. check if the request is not replayed (i.e. timestamp is not smaller than the latest known
    // submission ON THIS ENDPOINT - the liveness stream keeps its own mark, so the two cannot
    // invalidate each other's timestamps)
    let last_request = state
        .network_monitor_submissions
        .stress_submitted(body.body.signer)
        .await;

    // if we have no known requests, we might have just restarted
    // so we use the time of when we came back online - it's impossible there were any other requests since then
    let last_known = match last_request {
        Some(last) => last,
        None => {
            let uptime = state.api_status.uptime();
            OffsetDateTime::now_utc() - uptime
        }
    };

    if body.body.timestamp <= last_known {
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
        .set_stress_submitted(body.body.signer, body.body.timestamp)
        .await;

    // 6. process received results
    let signer = body.body.signer;
    let submitted = body.body.results.len();
    let mut mixnode_results = Vec::with_capacity(submitted);
    for result in body.body.results {
        if !result.is_mixnode {
            error!(
                %signer,
                node_id = result.node_id,
                "received a stress testing result for a non-mixnode entry which should never happen - is the nym-api outdated?"
            );
            continue;
        }
        if !(0.0..=1.0).contains(&result.test_performance) {
            error!(
                %signer,
                node_id = result.node_id,
                test_performance = result.test_performance,
                "received a stress testing result with performance outside the [0, 1] range - is the monitor misconfigured?"
            );
            continue;
        }
        mixnode_results.push(NymNodeStressTestingResult::from_submission(&signer, result));
    }

    // anything dropped above never reaches the database, so it must not be counted as a duplicate
    let attempted = mixnode_results.len();
    let rejected = submitted - attempted;

    let accepted = state
        .storage()
        .insert_nym_node_stress_testing_results(mixnode_results)
        .await? as usize;

    // insert-or-ignore means a batch can be fully accepted yet store nothing, so report the split
    // rather than leaving the submitter to infer it from a bare 200.
    let duplicates = attempted.saturating_sub(accepted);
    if duplicates > 0 {
        warn!(
            %signer,
            accepted,
            duplicates,
            "some submitted stress testing results were already stored and have been discarded - \
             expected when a batch is retried, but a persistently non-zero count means this \
             orchestrator's measurements are being dropped"
        );
    }

    Ok(Json(StressTestBatchSubmissionResponse {
        accepted: Some(accepted),
        duplicates: Some(duplicates),
        rejected: Some(rejected),
    }))
}

/// Accept a batch of liveness results from an authorised network monitor orchestrator.
///
/// Applies the SAME ordered validation as the stress endpoint above - staleness window, contract
/// membership of the signer, strict per-signer timestamp monotonicity, then signature - but against
/// its own replay high-water mark. The mark is what must not be shared: one orchestrator identity
/// signs both streams, so a single mark would have whichever stream posted second look replayed
/// forever. The checks themselves are deliberately repeated rather than abstracted over the two
/// body types, so that the order of a security-relevant sequence is readable in one place.
///
/// Unlike the stress endpoint, an entry for a gateway-capable node is expected rather than
/// dropped: liveness probes gateways as well as mixnodes, and the submitted score is a single
/// average whose shape is identical for both.
#[utoipa::path(
    tag = "Nym Nodes",
    post,
    path = "/liveness-testing/batch-submit",
    context_path = "/v3/nym-nodes",
    responses(
        (status = 200, body = LivenessTestBatchSubmissionResponse, description = "the submitted batch has been accepted, with a per-result breakdown of what was stored"),
        (status = 400, description = "the submitted request is stale or replayed"),
        (status = 401, description = "the submitted request was unauthorised or failed integrity check"),
        (status = 501, description = "this nym-api validates liveness batches but cannot yet store them"),
    ),
)]
async fn batch_submit_liveness_testing_results(
    State(state): State<AppState>,
    Json(body): Json<LivenessTestBatchSubmission>,
) -> ApiResult<Json<LivenessTestBatchSubmissionResponse>> {
    // 1. check if the request is not stale
    if body.body.is_stale(SUBMISSION_STALENESS_WINDOW) {
        return Err(AxumErrorResponse::bad_request(
            "request is stale, please resubmit it with a fresh timestamp",
        ));
    }

    // 2. check if the sent public key is even in the authorised set. the authorised set is shared
    // with the stress endpoint - one contract lists the orchestrators, and being authorised is not
    // per-kind
    if !state
        .network_monitors()
        .is_authorised(&state.nyxd_client, &body.body.signer)
        .await?
    {
        return Err(AxumErrorResponse::unauthorised(
            "the provided public key does not correspond to any known network monitor",
        ));
    }

    // 3. check if the request is not replayed, against THIS endpoint's mark only
    let last_request = state
        .network_monitor_submissions
        .liveness_submitted(body.body.signer)
        .await;

    // if we have no known requests, we might have just restarted
    // so we use the time of when we came back online - it's impossible there were any other requests since then
    let last_known = match last_request {
        Some(last) => last,
        None => {
            let uptime = state.api_status.uptime();
            OffsetDateTime::now_utc() - uptime
        }
    };

    if body.body.timestamp <= last_known {
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
        .set_liveness_submitted(body.body.signer, body.body.timestamp)
        .await;

    // 6. process received results
    //
    // Not yet implemented: the storage table this writes to arrives with task 11.3. Returning an
    // error rather than a `200` with zero counts is deliberate - the orchestrator only advances a
    // stream's watermark after a successful POST, so failing here makes it re-send these rows once
    // storage exists, whereas a bare success would silently drop every measurement in the batch.
    error!(
        signer = %body.body.signer,
        submitted = body.body.results.len(),
        "rejecting a validated liveness batch: this nym-api cannot store liveness results yet"
    );
    Err(AxumErrorResponse::not_implemented())
}

/// Report whether the given identity key is currently recognised by this nym-api as an
/// authorised network monitor orchestrator.
///
/// Intended for orchestrators to self-check after (re)announcing their key on-chain - a
/// successful response with `authorised: true` means this nym-api has picked up the chain change
/// and is ready to accept stress-test submissions signed by that key.
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
        .route("/known-monitors/{identity_key}", get(known_network_monitor))
}

/// Liveness has its own subtree because it has its own replay state; it carries no
/// `known-monitors` route of its own, since the authorised-orchestrator set is one contract-derived
/// set rather than one per test kind.
fn liveness_testing_routes() -> Router<AppState> {
    Router::new().route("/batch-submit", post(batch_submit_liveness_testing_results))
}

/// Build the `/v3/nym-nodes` subtree hosting the v3 network-monitor endpoints.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .nest("/stress-testing", stress_testing_routes())
        .nest("/liveness-testing", liveness_testing_routes())
}
