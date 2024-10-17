// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::middleware::auth::AuthLayer;
use crate::http::state::ApiState;
use axum::Router;
use nym_credential_proxy_requests::routes::api::v1;

// pub mod bandwidth_voucher;
// pub mod freepass;
pub mod openapi;
pub mod ticketbook;

pub(super) fn routes(auth_layer: AuthLayer) -> Router<ApiState> {
    // from docs:
    // ```
    // Note that the middleware is only applied to existing routes.
    // So you have to first add your routes (and / or fallback) and then call layer afterwards.
    // Additional routes added after layer is called will not have the middleware added.
    // ```
    // thus we first add relevant API routes, then the auth layer and finally the swagger routes
    Router::new()
        .nest(v1::TICKETBOOK, ticketbook::routes().route_layer(auth_layer))
        .merge(openapi::route())
}
