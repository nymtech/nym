use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::node_status_api::models::ErrorResponse;
use nym_api_requests::models::CirculatingSupplyResponse;
use rocket_okapi::openapi;

#[openapi(tag = "circulating-supply")]
#[get("/circulating-supply")]
pub(crate) async fn get_circulating_supply(
    cache: &State<CirculatingSupplyCache>,
) -> Result<Json<CirculatingSupplyResponse>, ErrorResponse> {
    match cache.get_circulating_supply().await {
        Some(value) => Ok(Json(value)),
        None => Err(ErrorResponse::new(
            "unavailable",
            Status::InternalServerError,
        )),
    }
}
