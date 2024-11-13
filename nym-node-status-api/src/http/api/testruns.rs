use axum::extract::DefaultBodyLimit;
use axum::Json;
use axum::{
    extract::{Path, State},
    Router,
};
use nym_common_models::ns_api::SubmitResults;
use nym_crypto::asymmetric::ed25519::PublicKey;
use reqwest::StatusCode;

use crate::db::models::TestRunStatus;
use crate::db::queries;
use crate::{
    db,
    http::{
        error::{HttpError, HttpResult},
        models::TestrunAssignment,
        state::AppState,
    },
};

// TODO dz consider adding endpoint to trigger testrun scan for a given gateway_id
// like in H< src/http/testruns.rs

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(request_testrun))
        .route("/:testrun_id", axum::routing::post(submit_testrun))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 5))
}

#[tracing::instrument(level = "debug", skip_all)]
async fn request_testrun(
    State(state): State<AppState>,
    body: String,
) -> HttpResult<Json<TestrunAssignment>> {
    // TODO dz log agent's network probe version
    let agent_pubkey = authenticate_agent(&body, &state)?;

    tracing::debug!("Agent {} requested testrun", agent_pubkey);

    let db = state.db_pool();
    let mut conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    return match db::queries::testruns::get_oldest_testrun_and_make_it_pending(&mut conn).await {
        Ok(res) => {
            if let Some(testrun) = res {
                tracing::info!(
                    "🏃‍ Assigned testrun row_id {} gateway {} to agent {}",
                    &testrun.testrun_id,
                    testrun.gateway_identity_key,
                    agent_pubkey
                );
                Ok(Json(testrun))
            } else {
                tracing::debug!("No testruns available for agent");
                Err(HttpError::no_testruns_available())
            }
        }
        Err(err) => Err(HttpError::internal_with_logging(err)),
    };
}

#[tracing::instrument(level = "debug", skip_all)]
async fn submit_testrun(
    Path(testrun_id): Path<i64>,
    State(state): State<AppState>,
    Json(probe_results): Json<SubmitResults>,
) -> HttpResult<StatusCode> {
    let agent_pubkey = authenticate_agent(&probe_results.public_key.to_base58_string(), &state)?;
    agent_pubkey
        .verify(&probe_results.message, &probe_results.signature)
        .map_err(|_| {
            tracing::warn!("Message verification failed, rejecting");
            HttpError::unauthorized()
        })?;

    let db = state.db_pool();
    let mut conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    let testrun = queries::testruns::get_in_progress_testrun_by_id(&mut conn, testrun_id)
        .await
        .map_err(|e| {
            tracing::error!("{e}");
            HttpError::not_found(testrun_id)
        })?;

    let gw_identity = db::queries::select_gateway_identity(&mut conn, testrun.gateway_id)
        .await
        .map_err(|_| {
            // should never happen:
            HttpError::internal_with_logging("No gateway found for testrun")
        })?;
    tracing::debug!(
        "Agent {} submitted testrun {} for gateway {} ({} bytes)",
        agent_pubkey,
        testrun_id,
        gw_identity,
        &probe_results.message.len(),
    );

    // TODO dz this should be part of a single transaction: commit after everything is done
    queries::testruns::update_testrun_status(&mut conn, testrun_id, TestRunStatus::Complete)
        .await
        .map_err(HttpError::internal_with_logging)?;
    queries::testruns::update_gateway_last_probe_log(
        &mut conn,
        testrun.gateway_id,
        &probe_results.message,
    )
    .await
    .map_err(HttpError::internal_with_logging)?;
    let result = get_result_from_log(&probe_results.message);
    queries::testruns::update_gateway_last_probe_result(&mut conn, testrun.gateway_id, &result)
        .await
        .map_err(HttpError::internal_with_logging)?;
    queries::testruns::update_gateway_score(&mut conn, testrun.gateway_id)
        .await
        .map_err(HttpError::internal_with_logging)?;

    tracing::info!(
        "✅ Testrun row_id {} for gateway {} complete",
        testrun.id,
        gw_identity
    );

    Ok(StatusCode::CREATED)
}

fn authenticate_agent(base58_pubkey: &str, state: &AppState) -> HttpResult<PublicKey> {
    let agent_pubkey = PublicKey::from_base58_string(base58_pubkey).map_err(|_| {
        if base58_pubkey.is_empty() {
            tracing::warn!("Auth key missing from request body, rejecting");
        } else {
            tracing::warn!("Failed to deserialize key from request body, rejecting");
        }
        HttpError::unauthorized()
    })?;
    if !state.is_registered(&agent_pubkey) {
        tracing::warn!("Public key {} not registered, rejecting", agent_pubkey);
        return Err(HttpError::unauthorized());
    }

    Ok(agent_pubkey)
}

fn get_result_from_log(log: &str) -> String {
    let re = regex::Regex::new(r"\n\{\s").unwrap();
    let result: Vec<_> = re.splitn(log, 2).collect();
    if result.len() == 2 {
        let res = format!("{} {}", "{", result[1]).to_string();
        return res;
    }
    "".to_string()
}
