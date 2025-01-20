// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::metrics::MetricsAppState;
use nym_node_metrics::NymNodeMetrics;
use nym_verloc::measurements::SharedVerlocStats;
use tokio::time::Instant;

pub mod metrics;

#[derive(Clone)]
pub struct AppState {
    pub(crate) startup_time: Instant,

    pub(crate) metrics: MetricsAppState,
}

impl AppState {
    #[allow(clippy::new_without_default)]
    pub fn new(metrics: NymNodeMetrics, verloc: SharedVerlocStats) -> Self {
        AppState {
            // is it 100% accurate?
            // no.
            // does it have to be?
            // also no.
            startup_time: Instant::now(),
            metrics: MetricsAppState { metrics, verloc },
        }
    }
}
