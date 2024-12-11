// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::AppState;
use axum::extract::FromRef;
use nym_node_metrics::NymNodeMetrics;

pub use nym_verloc::measurements::metrics::SharedVerlocStats;

#[derive(Clone)]
pub struct MetricsAppState {
    pub(crate) prometheus_access_token: Option<String>,

    pub(crate) metrics: NymNodeMetrics,

    pub(crate) verloc: SharedVerlocStats,
}

impl FromRef<AppState> for MetricsAppState {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.metrics.clone()
    }
}
