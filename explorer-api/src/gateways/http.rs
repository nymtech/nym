// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// due to the macro expansion of rather old rocket macros...
#![allow(unused_imports)]

use nym_explorer_api_requests::PrettyDetailedGatewayBond;
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use crate::state::ExplorerApiStateContext;

pub fn gateways_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: list]
}

#[openapi(tag = "gateways")]
#[get("/")]
pub(crate) async fn list(
    state: &State<ExplorerApiStateContext>,
) -> Result<Json<Vec<PrettyDetailedGatewayBond>>, NotFound<String>> {
    Ok(Json(state.inner.gateways.get_detailed_gateways().await))
}
