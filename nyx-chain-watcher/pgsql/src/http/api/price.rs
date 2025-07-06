use crate::db::models::PriceHistory;
use crate::db::queries::price::{get_average_price, get_latest_price};
use crate::http::error::Error;
use crate::http::error::HttpResult;
use crate::http::state::AppState;
use axum::{extract::State, Json, Router};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(price))
        .route("/average", axum::routing::get(average_price))
}

#[utoipa::path(
    tag = "NYM Price",
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

#[utoipa::path(
    tag = "NYM Price",
    get,
    path = "/v1/price/average",
    responses(
        (status = 200, body = String)
    )
)]
/// Fetch the average price cached by this API
async fn average_price(State(state): State<AppState>) -> HttpResult<Json<PriceHistory>> {
    get_average_price(state.db_pool())
        .await
        .map(Json::from)
        .map_err(|_| Error::internal())
}
