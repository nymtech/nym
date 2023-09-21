// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(info(title = "NymNode API"))]
pub(crate) struct ApiDoc;

pub(crate) fn route() -> Router {
    // provide absolute path to the openapi.json
    let config = utoipa_swagger_ui::Config::from("/api/v1/api-docs/openapi.json");
    SwaggerUi::new("/swagger")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
        .config(config)
        .into()
}
