use axum::extract::DefaultBodyLimit;
use axum::Json;
use axum::{
    extract::{Path, State},
    Router,
};
use nym_common_models::ns_api::{get_testrun, submit_results, VerifiableRequest};
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
    Json(request): Json<get_testrun::GetTestrunRequest>,
) -> HttpResult<Json<TestrunAssignment>> {
    // TODO dz log agent's network probe version
    authenticate(&request, &state)?;
    state
        .update_last_request_time(
            &request.payload.agent_public_key,
            &request.payload.timestamp,
        )
        .await?;

    let agent_pubkey = request.payload.agent_public_key;
    tracing::debug!("Agent {} requested testrun", agent_pubkey);

    let db = state.db_pool();
    let mut conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    if let Ok(testrun) =
        db::queries::testruns::testrun_in_progress_assigned_to_agent(&mut conn, &agent_pubkey).await
    {
        tracing::warn!(
            "Testrun {} already in progress for agent {:?}, rejecting",
            testrun.id,
            testrun.assigned_agent
        );
        return Err(HttpError::invalid_input(
            "Testrun already in progress for this agent",
        ));
    };

    return match db::queries::testruns::assign_oldest_testrun(&mut conn, agent_pubkey).await {
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
                tracing::debug!("No testruns available for agent {}", agent_pubkey);
                Err(HttpError::no_testruns_available())
            }
        }
        Err(err) => Err(HttpError::internal_with_logging(err)),
    };
}

#[tracing::instrument(level = "debug", skip_all)]
async fn submit_testrun(
    Path(submitted_testrun_id): Path<i64>,
    State(state): State<AppState>,
    Json(submitted_result): Json<submit_results::SubmitResults>,
) -> HttpResult<StatusCode> {
    authenticate(&submitted_result, &state)?;

    let db = state.db_pool();
    let mut conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    let submitter_pubkey = submitted_result.payload.agent_public_key;
    let assigned_testrun =
        queries::testruns::testrun_in_progress_assigned_to_agent(&mut conn, &submitter_pubkey)
            .await
            .map_err(|err| {
                tracing::warn!("No testruns in progress for agent {submitter_pubkey}: {err}");
                HttpError::invalid_input("Invalid testrun submitted")
            })?;
    if submitted_testrun_id != assigned_testrun.id {
        tracing::warn!(
            "Agent {} submitted testrun {} but {} was expected",
            submitter_pubkey,
            submitted_testrun_id,
            assigned_testrun.id
        );
        return Err(HttpError::invalid_input("Invalid testrun submitted"));
    }
    if Some(submitted_result.payload.assigned_at_utc) != assigned_testrun.last_assigned_utc {
        tracing::warn!(
            "Submitted testrun timestamp mismatch: {} != {:?}, rejecting",
            submitted_result.payload.assigned_at_utc,
            assigned_testrun.last_assigned_utc
        );
        return Err(HttpError::invalid_input("Invalid testrun submitted"));
    }

    let gw_identity = db::queries::select_gateway_identity(&mut conn, assigned_testrun.gateway_id)
        .await
        .map_err(|_| {
            // should never happen:
            HttpError::internal_with_logging(format!(
                "No gateway found for testrun {submitted_testrun_id}"
            ))
        })?;
    tracing::debug!(
        "Agent {} submitted testrun {} for gateway {} ({} bytes)",
        submitter_pubkey,
        submitted_testrun_id,
        gw_identity,
        &submitted_result.payload.probe_result.len(),
    );

    queries::testruns::update_testrun_status(
        &mut conn,
        submitted_testrun_id,
        TestRunStatus::Complete,
    )
    .await
    .map_err(HttpError::internal_with_logging)?;
    queries::testruns::update_gateway_last_probe_log(
        &mut conn,
        assigned_testrun.gateway_id,
        &submitted_result.payload.probe_result,
    )
    .await
    .map_err(HttpError::internal_with_logging)?;
    let result = get_result_from_log(&submitted_result.payload.probe_result);
    queries::testruns::update_gateway_last_probe_result(
        &mut conn,
        assigned_testrun.gateway_id,
        &result,
    )
    .await
    .map_err(HttpError::internal_with_logging)?;
    queries::testruns::update_gateway_score(&mut conn, assigned_testrun.gateway_id)
        .await
        .map_err(HttpError::internal_with_logging)?;

    tracing::info!(
        "✅ Testrun row_id {} for gateway {} complete",
        assigned_testrun.id,
        gw_identity
    );

    Ok(StatusCode::CREATED)
}

// TODO dz this should be middleware
#[tracing::instrument(level = "debug", skip_all)]
fn authenticate(request: &impl VerifiableRequest, state: &AppState) -> HttpResult<()> {
    if !state.is_registered(request.public_key()) {
        tracing::warn!("Public key not registered with NS API, rejecting");
        return Err(HttpError::unauthorized());
    };

    request.verify_signature().map_err(|_| {
        tracing::warn!("Signature verification failed, rejecting");
        HttpError::unauthorized()
    })?;

    Ok(())
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
