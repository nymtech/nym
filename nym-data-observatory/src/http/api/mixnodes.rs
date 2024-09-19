use axum::{extract::State, Json, Router};

use crate::http::{error::HttpResult, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::get(mixnodes))
}

#[utoipa::path(
    tag = "Mixnodes",
    get,
    path = "/v1/mixnodes",
    responses(
        (status = 200, body = String)
    )
)]
async fn mixnodes(State(_state): State<AppState>) -> HttpResult<Json<serde_json::Value>> {
    Ok(Json(
        serde_json::json!({"message": "😎 Nothing to see here, move along 😎"}),
    ))
}
