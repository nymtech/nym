// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use okapi::openapi3::OpenApi;
use rocket_okapi::swagger_ui::SwaggerUIConfig;

pub fn custom_openapi_spec() -> OpenApi {
    use rocket_okapi::okapi::openapi3::*;
    OpenApi {
        openapi: OpenApi::default_version(),
        info: Info {
            title: "Nym API".to_owned(),
            description: None,
            terms_of_service: None,
            contact: None,
            license: None,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            ..Default::default()
        },
        servers: get_servers(),
        ..Default::default()
    }
}

fn get_servers() -> Vec<rocket_okapi::okapi::openapi3::Server> {
    if std::env::var_os("CARGO").is_some() {
        return vec![];
    }
    vec![rocket_okapi::okapi::openapi3::Server {
        url: std::env::var("OPEN_API_BASE").unwrap_or_else(|_| "/api/v1/".to_owned()),
        description: Some("API".to_owned()),
        ..Default::default()
    }]
}

pub(crate) fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "../v1/openapi.json".to_owned(),
        ..SwaggerUIConfig::default()
    }
}
