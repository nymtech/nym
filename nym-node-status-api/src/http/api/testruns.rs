use axum::extract::DefaultBodyLimit;
use axum::Json;
use axum::{
    extract::{Path, State},
    Router,
};
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
async fn request_testrun(State(state): State<AppState>) -> HttpResult<Json<TestrunAssignment>> {
    // TODO dz log agent's key
    // TODO dz log agent's network probe version
    tracing::debug!("Agent requested testrun");

    let db = state.db_pool();
    let mut conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    return match db::queries::testruns::get_oldest_testrun_and_make_it_pending(&mut conn).await {
        Ok(res) => {
            if let Some(testrun) = res {
                tracing::debug!(
                    "🏃‍ Assigned testrun row_id {} gateway {} to agent",
                    &testrun.testrun_id,
                    testrun.gateway_identity_key
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

// TODO dz accept testrun_id as query parameter
#[tracing::instrument(level = "debug", skip_all)]
async fn submit_testrun(
    Path(testrun_id): Path<i64>,
    State(state): State<AppState>,
    body: String,
) -> HttpResult<StatusCode> {
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
        "Agent submitted testrun {} for gateway {} ({} bytes)",
        testrun_id,
        gw_identity,
        body.len(),
    );

    // TODO dz this should be part of a single transaction: commit after everything is done
    queries::testruns::update_testrun_status(&mut conn, testrun_id, TestRunStatus::Complete)
        .await
        .map_err(HttpError::internal_with_logging)?;
    queries::testruns::update_gateway_last_probe_log(&mut conn, testrun.gateway_id, &body)
        .await
        .map_err(HttpError::internal_with_logging)?;
    let result = get_result_from_log(&body);
    queries::testruns::update_gateway_last_probe_result(&mut conn, testrun.gateway_id, &result)
        .await
        .map_err(HttpError::internal_with_logging)?;
    queries::testruns::update_gateway_score(&mut conn, testrun.gateway_id)
        .await
        .map_err(HttpError::internal_with_logging)?;
    // TODO dz log gw identity key

    tracing::info!(
        "✅ Testrun row_id {} for gateway {} complete",
        testrun.id,
        gw_identity
    );

    Ok(StatusCode::CREATED)
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
