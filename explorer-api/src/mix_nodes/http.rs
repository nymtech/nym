use crate::mix_node::models::PrettyDetailedMixNodeBond;
use crate::state::ExplorerApiStateContext;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

pub fn mix_nodes_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: list]
}

#[openapi(tag = "mix_nodes")]
#[get("/")]
pub(crate) async fn list(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedMixNodeBond>> {
    Json(state.inner.mix_nodes.get_mixnodes_with_location().await)
}
