// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::state::ExplorerApiStateContext;
use nym_explorer_api_requests::PrettyDetailedGatewayBond;
use okapi::openapi3::OpenApi;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::settings::OpenApiSettings;

pub fn unstable_temp_nymnodes_make_default_routes(
    settings: &OpenApiSettings,
) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: all_gateways]
}

#[openapi(tag = "UNSTABLE")]
#[get("/gateways")]
pub(crate) async fn all_gateways(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedGatewayBond>> {
    let mut gateways = state.inner.gateways.get_detailed_gateways().await;
    gateways.append(&mut state.inner.nymnodes.pretty_gateways().await);

    Json(gateways)
}
