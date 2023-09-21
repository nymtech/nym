// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::router::api;
use crate::http::state::AppState;
use axum::Router;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(title = "NymNode API"),
    paths(api::v1::build_info::build_info,),
    components(schemas(api::Output, api::OutputParams, BinaryBuildInformationOwned))
)]
pub(crate) struct ApiDoc;

pub(crate) fn route() -> Router<AppState> {
    // provide absolute path to the openapi.json
    let config = utoipa_swagger_ui::Config::from("/api/v1/api-docs/openapi.json");
    SwaggerUi::new("/swagger")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .config(config)
        .into()
}
