use axum::{extract::State, Json, Router};
use nym_validator_client::models::BinaryBuildInformationOwned;
use tracing::instrument;

use crate::http::{
    error::HttpResult,
    state::{AppState, HealthInfo},
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/build_information", axum::routing::get(build_information))
        .route("/health", axum::routing::get(health))
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/build_information",
    context_path = "/v2/status",
    responses(
        (status = 200, body = BinaryBuildInformationOwned)
    )
)]
#[instrument(level = tracing::Level::INFO, skip_all)]
async fn build_information(
    State(state): State<AppState>,
) -> HttpResult<Json<BinaryBuildInformationOwned>> {
    let build_info = state.build_information().to_owned();

    Ok(Json(build_info))
}

#[utoipa::path(
    tag = "Status",
    get,
    path = "/health",
    context_path = "/v2/status",
    responses(
        (status = 200, body = HealthInfo)
    )
)]
#[instrument(level = tracing::Level::INFO, skip_all)]
async fn health(State(state): State<AppState>) -> HttpResult<Json<HealthInfo>> {
    Ok(Json(state.health()))
}
