// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::api::v1::metrics::mixing::mixing_stats;
use crate::state::metrics::MetricsAppState;
use axum::extract::FromRef;
use axum::routing::get;
use axum::Router;
use nym_node_requests::routes::api::v1::metrics;

pub mod mixing;

#[derive(Debug, Clone, Default)]
pub struct Config {
    //
}

pub(super) fn routes<S>(_config: Config) -> Router<S>
where
    S: Send + Sync + 'static + Clone,
    MetricsAppState: FromRef<S>,
{
    Router::new().route(metrics::MIXING, get(mixing_stats))
}
