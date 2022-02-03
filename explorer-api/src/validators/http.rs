// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use crate::state::ExplorerApiStateContext;
use crate::validators::models::PrettyValidatorInfo;

pub fn validators_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: list]
}

#[openapi(tag = "validators")]
#[get("/")]
pub(crate) async fn list(
    state: &State<ExplorerApiStateContext>,
) -> Result<Json<Vec<PrettyValidatorInfo>>, NotFound<String>> {
    Ok(Json(state.inner.validators.get_validators().await))
}
