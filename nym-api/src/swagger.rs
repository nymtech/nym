// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rocket_okapi::swagger_ui::SwaggerUIConfig;

pub(crate) fn get_docs() -> SwaggerUIConfig {
    SwaggerUIConfig {
        url: "../v1/openapi.json".to_owned(),
        ..SwaggerUIConfig::default()
    }
}
