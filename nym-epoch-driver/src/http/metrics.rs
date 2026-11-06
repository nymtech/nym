// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use axum::routing::get;
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_metrics::metrics;
use std::sync::Arc;
use zeroize::Zeroizing;

pub(super) fn routes(bearer_token: String) -> Router {
    Router::new().route(
        "/prometheus",
        get(prometheus_metrics).route_layer(AuthLayer::new(Arc::new(Zeroizing::new(bearer_token)))),
    )
}

/// Returns `prometheus` compatible metrics
// the AuthLayer is protecting access to this endpoint
async fn prometheus_metrics() -> String {
    metrics!()
}
