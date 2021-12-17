use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use crate::mix_node::http::PrettyMixNodeBondWithLocation;
use crate::state::ExplorerApiStateContext;

pub fn mix_nodes_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: list]
}

#[openapi(tag = "mix_nodes")]
#[get("/")]
pub(crate) async fn list(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyMixNodeBondWithLocation>> {
    Json(state.inner.mix_nodes.get_mixnodes_with_location().await)
}
