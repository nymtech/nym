use axum::{extract::State, Router};
use reqwest::StatusCode;

use crate::http::{error::HttpResult, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::post(submit))
}

#[tracing::instrument(level = "debug", skip_all)]
async fn submit(State(_state): State<AppState>, body: String) -> HttpResult<StatusCode> {
    tracing::debug!(
        "Agent submitted probe results. Total length: {}",
        body.len(),
    );
    // TODO dz store testrun results

    // let db = state.db_pool();
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
