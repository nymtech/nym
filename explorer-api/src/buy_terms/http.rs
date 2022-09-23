// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::geo_ip::location::Location;
use crate::state::ExplorerApiStateContext;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::settings::OpenApiSettings;

pub fn nym_terms_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: terms]
}

#[openapi(tag = "terms")]
#[get("/")]
pub(crate) async fn terms(
    _state: &State<ExplorerApiStateContext>,
    location: Location,
) -> Result<Json<String>, status::Forbidden<String>> {
    if location.iso_alpha2 == "US" {
        return Err(status::Forbidden(Some("US government sucks".to_string())));
    }
    Ok(Json("Nym Terms & Conditions: Welcome".to_string()))
}
