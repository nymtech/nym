use axum::{extract::State, Json, Router};
use tracing::instrument;

use crate::{
    db::models::NetworkSummary,
    http::{error::HttpResult, models::SummaryHistory, state::AppState},
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(summary))
        .route("/history", axum::routing::get(summary_history))
}

#[utoipa::path(
    tag = "Summary",
    get,
    path = "/v2/summary",
    responses(
        (status = 200, body = NetworkSummary)
    )
)]
#[instrument(level = tracing::Level::DEBUG, skip_all)]
async fn summary(State(state): State<AppState>) -> HttpResult<Json<NetworkSummary>> {
    crate::db::queries::get_summary(state.db_pool())
        .await
        .map(Json)
}

#[utoipa::path(
    tag = "Summary",
    get,
    path = "/v2/summary/history",
    responses(
        (status = 200, body = Vec<SummaryHistory>)
    )
)]
#[instrument(level = tracing::Level::DEBUG, skip_all)]
async fn summary_history(State(state): State<AppState>) -> HttpResult<Json<Vec<SummaryHistory>>> {
    Ok(Json(
        state.cache().get_summary_history(state.db_pool()).await,
    ))
}
