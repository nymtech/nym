// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::api::v1::metrics::packets_stats::packets_stats;
use crate::node::http::api::v1::metrics::prometheus::prometheus_metrics;
use crate::node::http::api::v1::metrics::sessions::sessions_stats;
use crate::node::http::api::v1::metrics::verloc::verloc_stats;
use crate::node::http::api::v1::metrics::wireguard::wireguard_stats;
use crate::node::http::state::metrics::MetricsAppState;
use axum::extract::FromRef;
use axum::routing::get;
use axum::Router;
use nym_http_api_common::middleware::bearer_auth::AuthLayer;
use nym_node_requests::routes::api::v1::metrics;
use nym_node_requests::routes::api::v1::metrics::prometheus_absolute;
use std::sync::Arc;
use tracing::info;
use zeroize::Zeroizing;

pub mod legacy_mixing;
pub mod packets_stats;
pub mod prometheus;
pub mod sessions;
pub mod verloc;
pub mod wireguard;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub bearer_token: Option<Arc<Zeroizing<String>>>,
}

#[allow(deprecated)]
pub(super) fn routes<S>(config: Config) -> Router<S>
where
    S: Send + Sync + 'static + Clone,
    MetricsAppState: FromRef<S>,
{
    if config.bearer_token.is_none() {
        info!(
            "bearer token hasn't been set. '{}' route will not be exposed",
            prometheus_absolute()
        )
    }

    let router = Router::new()
        .route(
            metrics::LEGACY_MIXING,
            get(legacy_mixing::legacy_mixing_stats),
        )
        .route(metrics::PACKETS_STATS, get(packets_stats))
        .route(metrics::WIREGUARD_STATS, get(wireguard_stats))
        .route(metrics::SESSIONS, get(sessions_stats))
        .route(metrics::VERLOC, get(verloc_stats));

    let auth_middleware = config.bearer_token.map(AuthLayer::new);

    // don't expose prometheus route without bearer token set
    if let Some(auth_middleware) = auth_middleware {
        router.route(
            metrics::PROMETHEUS,
            get(prometheus_metrics).route_layer(auth_middleware),
        )
    } else {
        router
    }
}
