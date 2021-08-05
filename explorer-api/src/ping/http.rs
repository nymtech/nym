use rocket::serde::json::Json;
use rocket::Route;
use serde::Deserialize;
use serde::Serialize;

pub fn ping_make_default_routes() -> Vec<Route> {
    routes_with_openapi![index]
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub(crate) struct PingResponse {
    response_time: u32,
}

#[openapi(tag = "ping")]
#[get("/")]
pub(crate) async fn index() -> Json<PingResponse> {
    Json(PingResponse { response_time: 42 })
}
