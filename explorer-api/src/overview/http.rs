use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use crate::mix_nodes::http::get_mixnode_summary;
use crate::overview::models::{GatewaySummary, OverviewSummary, ValidatorSummary};
use crate::state::ExplorerApiStateContext;

pub fn overview_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: summary]
}

#[openapi(tag = "overview")]
#[get("/summary")]
pub(crate) async fn summary(state: &State<ExplorerApiStateContext>) -> Json<OverviewSummary> {
    Json(OverviewSummary {
        mixnodes: get_mixnode_summary(state).await,
        validators: ValidatorSummary { count: 0 }, // TODO: implement
        gateways: GatewaySummary { count: 0 },     // TODO: implement
    })
}
