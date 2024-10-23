use axum::Json;
use axum::{
    extract::{Path, State},
    Router,
};
use reqwest::StatusCode;

use crate::{
    db,
    http::{
        error::{HttpError, HttpResult},
        models::TestrunAssignment,
        state::AppState,
    },
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(request_testrun))
        .route("/{testrun_id}", axum::routing::post(submit_testrun))
}

#[tracing::instrument(level = "debug", skip_all)]
async fn request_testrun(State(state): State<AppState>) -> HttpResult<Json<TestrunAssignment>> {
    // TODO dz log agent's key
    tracing::debug!("Agent requested testrun",);
    // TODO dz store testrun results

    let db = state.db_pool();
    let conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    return match db::queries::testruns::get_oldest_testrun(conn).await {
        Ok(res) => {
            if let Some(testrun) = res {
                Ok(Json(testrun.into()))
            } else {
                Err(HttpError::not_found("No testruns available"))
            }
        }
        Err(err) => Err(HttpError::internal_with_logging(err)),
    };
}

// TODO dz accept testrun_id as query parameter
#[tracing::instrument(level = "debug", skip_all)]
async fn submit_testrun(
    Path(testrun_id): Path<u32>,
    State(state): State<AppState>,
    body: String,
) -> HttpResult<StatusCode> {
    tracing::debug!(
        "Agent submitted testrun {}. Total length: {}",
        testrun_id,
        body.len(),
    );
    // TODO dz store testrun results

    let db = state.db_pool();
    let _conn = db
        .acquire()
        .await
        .map_err(HttpError::internal_with_logging)?;

    Ok(StatusCode::CREATED)
}
