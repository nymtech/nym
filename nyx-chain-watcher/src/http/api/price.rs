use crate::db::models::PriceHistory;
use crate::db::queries::price::get_latest_price;
use crate::http::error::Error;
use crate::http::error::HttpResult;
use crate::http::state::AppState;
use axum::{extract::State, Json, Router};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::get(price))
}

#[utoipa::path(
    tag = "Nym Price",
    get,
    path = "/v1/price",
    responses(
        (status = 200, body = String)
    )
)]

/// Fetch the latest price cached by this API
async fn price(State(state): State<AppState>) -> HttpResult<Json<PriceHistory>> {
    get_latest_price(state.db_pool())
        .await
        .map(Json::from)
        .map_err(|_| Error::internal())
}
