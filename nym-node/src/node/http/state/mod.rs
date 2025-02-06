// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::state::load::CachedNodeLoad;
use crate::node::http::state::metrics::MetricsAppState;
use nym_node_metrics::NymNodeMetrics;
use nym_verloc::measurements::SharedVerlocStats;
use std::time::Duration;
use tokio::time::Instant;

pub mod load;
pub mod metrics;

#[derive(Clone)]
pub struct AppState {
    pub(crate) startup_time: Instant,

    pub(crate) cached_load: CachedNodeLoad,

    pub(crate) metrics: MetricsAppState,
}

impl AppState {
    #[allow(clippy::new_without_default)]
    pub fn new(
        metrics: NymNodeMetrics,
        verloc: SharedVerlocStats,
        load_cache_ttl: Duration,
    ) -> Self {
        AppState {
            // is it 100% accurate?
            // no.
            // does it have to be?
            // also no.
            startup_time: Instant::now(),
            cached_load: CachedNodeLoad::new(load_cache_ttl),
            metrics: MetricsAppState { metrics, verloc },
        }
    }
}
