use crate::http::error::HttpResult;
use crate::http::state::AppState;
use axum::extract::State;
use axum::{Json, Router};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/addresses", axum::routing::get(get_addresses))
}

#[utoipa::path(
    tag = "Watcher Configuration",
    get,
    path = "/v1/watcher/addresses",
    responses(
        (status = 200, body = Vec<String>)
    )
)]

/// Fetch the addresses being watched by the chain watcher
async fn get_addresses(State(state): State<AppState>) -> HttpResult<Json<Vec<String>>> {
    Ok(Json(state.watched_addresses.clone()))
}
