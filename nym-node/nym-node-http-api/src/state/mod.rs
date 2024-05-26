// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::metrics::{MetricsAppState, SharedMixingStats, SharedVerlocStats};
use tokio::time::Instant;

pub mod metrics;

#[derive(Debug, Clone)]
pub struct AppState {
    pub(crate) startup_time: Instant,

    pub(crate) metrics: MetricsAppState,
}

impl AppState {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        AppState {
            // is it 100% accurate?
            // no.
            // does it have to be?
            // also no.
            startup_time: Instant::now(),
            metrics: Default::default(),
        }
    }

    #[must_use]
    pub fn with_mixing_stats(mut self, mixing_stats: SharedMixingStats) -> Self {
        self.metrics.mixing_stats = mixing_stats;
        self
    }

    #[must_use]
    pub fn with_verloc_stats(mut self, verloc_stats: SharedVerlocStats) -> Self {
        self.metrics.verloc = verloc_stats;
        self
    }
}
