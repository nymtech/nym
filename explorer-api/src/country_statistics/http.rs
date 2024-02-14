use crate::country_statistics::country_nodes_distribution::CountryNodesDistribution;
use crate::state::ExplorerApiStateContext;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

pub fn country_statistics_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: index]
}

// We could either separate stuff by structure (like this, http is separate), or we could just
// stick the http route directly into each sub-application (e.g. put this file into the
// "country_statistics" module directly
#[openapi(tag = "country_statistics")]
#[get("/")]
pub(crate) async fn index(
    state: &State<ExplorerApiStateContext>,
) -> Json<CountryNodesDistribution> {
    Json(state.inner.country_node_distribution.get_all().await)
}
