use crate::db::models::{TestRunDto, TestRunStatus};
use crate::db::queries;
use crate::db::DbConnection;
use crate::utils::{now_utc, unix_timestamp_to_utc_rfc3339};
use crate::{
    db,
    http::{
        error::{HttpError, HttpResult},
        models::TestrunAssignment,
        state::AppState,
    },
};
use axum::extract::DefaultBodyLimit;
use axum::Json;
use axum::{
    extract::{Path, State},
    Router,
};
use nym_node_status_client::{
    auth::VerifiableRequest,
    models::{get_testrun, submit_results, submit_results_v2},
};
use reqwest::StatusCode;
use tracing::warn;

// TODO dz consider adding endpoint to trigger testrun scan for a given gateway_id
// like in H< src/http/testruns.rs

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(request_testrun))
        .route("/:testrun_id", axum::routing::post(submit_testrun))
        .route("/:testrun_id/v2", axum::routing::post(submit_testrun_v2))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 5))
}

#[tracing::instrument(level = "debug", skip_all)]
async fn request_testrun(
    State(state): State<AppState>,
    Json(request): Json<get_testrun::GetTestrunRequest>,
) -> HttpResult<Json<TestrunAssignment>> {
    // TODO dz log agent's network probe version
    authenticate(&request, &state)?;
    is_fresh(&request.payload.timestamp)?;

    tracing::debug!("Agent requested testrun");

    let db = state.db_pool();
    let mut conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    let active_testruns = db::queries::testruns::count_testruns_in_progress(&mut conn)
        .await
        .map_err(HttpError::internal_with_logging)?;
    if active_testruns >= state.agent_max_count() {
        tracing::warn!(
            "{}/{} testruns in progress, rejecting",
            active_testruns,
            state.agent_max_count()
        );
        return Err(HttpError::no_testruns_available());
    }

    return match db::queries::testruns::assign_oldest_testrun(&mut conn).await {
        Ok(res) => {
            if let Some(testrun) = res {
                tracing::info!(
                    "🏃‍ Assigned testrun row_id {} gateway {} to agent",
                    &testrun.testrun_id,
                    testrun.gateway_identity_key,
                );
                Ok(Json(testrun))
            } else {
                tracing::debug!("No testruns available");
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

    let assigned_testrun =
        queries::testruns::get_in_progress_testrun_by_id(&mut conn, submitted_testrun_id)
            .await
            .map_err(|err| {
                tracing::warn!(
                    "No testruns in progress for testrun_id {}: {}",
                    submitted_testrun_id,
                    err
                );
                HttpError::invalid_input("Invalid testrun submitted")
            })?;
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
        "Agent submitted testrun {} for gateway {} ({} bytes)",
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
        submitted_result.payload.probe_result.clone(),
    )
    .await
    .map_err(HttpError::internal_with_logging)?;
    let result = get_result_from_log(&submitted_result.payload.probe_result);
    queries::testruns::update_gateway_last_probe_result(
        &mut conn,
        assigned_testrun.gateway_id,
        result,
    )
    .await
    .map_err(HttpError::internal_with_logging)?;
    queries::testruns::update_gateway_score(&mut conn, assigned_testrun.gateway_id)
        .await
        .map_err(HttpError::internal_with_logging)?;

    let created_at = unix_timestamp_to_utc_rfc3339(assigned_testrun.created_utc);
    let last_assigned = assigned_testrun
        .last_assigned_utc
        .map(unix_timestamp_to_utc_rfc3339)
        .unwrap_or_else(|| String::from("never"));
    tracing::info!(
        gateway_id = gw_identity,
        last_assigned = last_assigned,
        created_at = created_at,
        "✅ Testrun row_id {} for gateway complete",
        assigned_testrun.id,
    );

    Ok(StatusCode::CREATED)
}

#[tracing::instrument(level = "debug", skip_all)]
async fn submit_testrun_v2(
    Path(submitted_testrun_id): Path<i64>,
    State(state): State<AppState>,
    Json(submission): Json<submit_results_v2::SubmitResultsV2>,
) -> HttpResult<StatusCode> {
    authenticate(&submission, &state)?;
    is_fresh(&submission.payload.assigned_at_utc)?;

    let db = state.db_pool();
    let mut conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    // Try to find existing testrun
    match queries::testruns::get_testrun_by_id(&mut conn, submitted_testrun_id).await {
        Ok(testrun) => {
            // Validate it matches the submission
            let gw_identity = queries::select_gateway_identity(&mut conn, testrun.gateway_id)
                .await
                .map_err(HttpError::internal_with_logging)?;

            if gw_identity != submission.payload.gateway_identity_key {
                tracing::warn!(
                    "Gateway mismatch for testrun {}: expected {}, got {}",
                    submitted_testrun_id,
                    gw_identity,
                    submission.payload.gateway_identity_key
                );
                return Err(HttpError::invalid_input("Gateway identity mismatch"));
            }

            // Process normally using existing testrun
            process_testrun_submission(testrun, submission.payload, &mut conn).await
        }
        Err(_) => {
            // External testrun - create records
            tracing::info!(
                "Creating external testrun {} for gateway {}",
                submitted_testrun_id,
                submission.payload.gateway_identity_key
            );

            // Get or create gateway
            let gateway_id =
                queries::get_or_create_gateway(&mut conn, &submission.payload.gateway_identity_key)
                    .await
                    .map_err(HttpError::internal_with_logging)?;

            // Create testrun
            queries::testruns::insert_external_testrun(
                &mut conn,
                submitted_testrun_id,
                gateway_id,
                submission.payload.assigned_at_utc,
            )
            .await
            .map_err(HttpError::internal_with_logging)?;

            // Process submission
            process_testrun_submission_by_gateway(gateway_id, submission.payload, &mut conn).await
        }
    }
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

static FRESHNESS_CUTOFF: time::Duration = time::Duration::minutes(1);

fn is_fresh(request_time: &i64) -> HttpResult<()> {
    // if a request took longer than N minutes to reach NS API, something is very wrong
    let request_time = time::UtcDateTime::from_unix_timestamp(*request_time).map_err(|e| {
        warn!("Failed to parse request time: {e}");
        HttpError::unauthorized()
    })?;

    let cutoff_timestamp = now_utc() - FRESHNESS_CUTOFF;
    if request_time < cutoff_timestamp {
        warn!("Request older than {}s, rejecting", cutoff_timestamp);
        return Err(HttpError::unauthorized());
    }
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

async fn process_testrun_submission(
    testrun: TestRunDto,
    payload: submit_results_v2::Payload,
    conn: &mut DbConnection,
) -> HttpResult<StatusCode> {
    // Validate timestamp matches
    if Some(payload.assigned_at_utc) != testrun.last_assigned_utc {
        tracing::warn!(
            "Submitted testrun timestamp mismatch: {} != {:?}, rejecting",
            payload.assigned_at_utc,
            testrun.last_assigned_utc
        );
        return Err(HttpError::invalid_input("Invalid testrun submitted"));
    }

    // Process the submission
    process_testrun_submission_by_gateway(testrun.gateway_id, payload, conn).await
}

async fn process_testrun_submission_by_gateway(
    gateway_id: i64,
    payload: submit_results_v2::Payload,
    conn: &mut DbConnection,
) -> HttpResult<StatusCode> {
    let gw_identity = &payload.gateway_identity_key;

    tracing::debug!(
        "Processing testrun submission for gateway {} ({} bytes)",
        gw_identity,
        payload.probe_result.len(),
    );

    // Update testrun status to complete
    queries::testruns::update_testrun_status_by_gateway(conn, gateway_id, TestRunStatus::Complete)
        .await
        .map_err(HttpError::internal_with_logging)?;

    // Update gateway with results
    queries::testruns::update_gateway_last_probe_log(
        conn,
        gateway_id,
        payload.probe_result.clone(),
    )
    .await
    .map_err(HttpError::internal_with_logging)?;

    let result = get_result_from_log(&payload.probe_result);
    queries::testruns::update_gateway_last_probe_result(conn, gateway_id, result)
        .await
        .map_err(HttpError::internal_with_logging)?;

    queries::testruns::update_gateway_score(conn, gateway_id)
        .await
        .map_err(HttpError::internal_with_logging)?;

    let assigned_at = unix_timestamp_to_utc_rfc3339(payload.assigned_at_utc);
    let now = now_utc();
    tracing::info!(
        "✅ Testrun for gateway {} complete (assigned at {}, current time {})",
        gw_identity,
        assigned_at,
        now
    );

    Ok(StatusCode::CREATED)
}
