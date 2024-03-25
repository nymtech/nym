// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::metrics::mixing::mixing_stats;
use crate::api::v1::metrics::prometheus::prometheus_metrics;
use crate::api::v1::metrics::verloc::verloc_stats;
use crate::state::metrics::MetricsAppState;
use axum::extract::FromRef;
use axum::routing::get;
use axum::Router;
use nym_node_requests::routes::api::v1::metrics;

pub mod mixing;
pub mod prometheus;
pub mod verloc;

#[derive(Debug, Clone, Default)]
pub struct Config {
    //
}

pub(super) fn routes<S>(_config: Config) -> Router<S>
where
    S: Send + Sync + 'static + Clone,
    MetricsAppState: FromRef<S>,
{
    Router::new()
        .route(metrics::MIXING, get(mixing_stats))
        .route(metrics::VERLOC, get(verloc_stats))
        .route(metrics::PROMETHEUS, get(prometheus_metrics))
}
