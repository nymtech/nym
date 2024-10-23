use axum::{
    extract::{Path, State},
    Router,
};
use reqwest::StatusCode;

use crate::{
    db,
    http::{
        error::{HttpError, HttpResult},
        state::AppState,
    },
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/{testrun_id}", axum::routing::post(submit))
}

// TODO dz accept testrun_id as query parameter
#[tracing::instrument(level = "debug", skip_all)]
async fn submit(
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

    // let testruns =

    // db::queries::testruns::update_status(conn, task_id, status)

    // let res = state.cache().get_gateway_list(db).await;
    // let res: Vec<GatewaySkinny> = res
    //     .iter()
    //     .filter(|g| g.bonded)
    //     .map(|g| GatewaySkinny {
    //         gateway_identity_key: g.gateway_identity_key.clone(),
    //         self_described: g.self_described.clone(),
    //         performance: g.performance,
    //         explorer_pretty_bond: g.explorer_pretty_bond.clone(),
    //         last_probe_result: g.last_probe_result.clone(),
    //         last_testrun_utc: g.last_testrun_utc.clone(),
    //         last_updated_utc: g.last_updated_utc.clone(),
    //         routing_score: g.routing_score,
    //         config_score: g.config_score,
    //     })
    //     .collect();

    Ok(StatusCode::CREATED)
}
