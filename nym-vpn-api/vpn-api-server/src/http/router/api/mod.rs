// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::ApiState;
use axum::Router;
use nym_credential_proxy_requests::routes;

use crate::http::middleware::auth::AuthLayer;
pub(crate) use nym_http_api_common::{Output, OutputParams};

pub mod v1;

pub(super) fn routes(auth_layer: AuthLayer) -> Router<ApiState> {
    Router::new().nest(routes::api::V1, v1::routes(auth_layer))
}
