use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use crate::mix_nodes::http::get_mixnode_summary;
use crate::overview::models::OverviewSummary;
use crate::state::ExplorerApiStateContext;

pub fn overview_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: summary]
}

#[openapi(tag = "overview")]
#[get("/summary")]
pub(crate) async fn summary(state: &State<ExplorerApiStateContext>) -> Json<OverviewSummary> {
    Json(OverviewSummary {
        mixnodes: get_mixnode_summary(state).await,
        validators: state.inner.validators.get_validator_summary().await,
        gateways: state.inner.gateways.get_gateway_summary().await,
    })
}
