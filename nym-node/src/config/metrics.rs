// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsConfig {
    #[serde(default)]
    pub debug: Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debug {
    /// Specify whether running statistics of this node should be logged to the console.
    pub log_stats_to_console: bool,

    /// Specify the rate of which the metrics aggregator should call the `on_update` methods of all its registered handlers.
    #[serde(with = "humantime_serde")]
    pub aggregator_update_rate: Duration,

    /// Specify the target rate of clearing old stale mixnet metrics.
    #[serde(with = "humantime_serde")]
    pub stale_mixnet_metrics_cleaner_rate: Duration,

    /// Specify the target rate of updating global prometheus counters.
    #[serde(with = "humantime_serde")]
    pub global_prometheus_counters_update_rate: Duration,

    /// Specify the target rate of updating egress packets pending delivery counter.
    #[serde(with = "humantime_serde")]
    pub pending_egress_packets_update_rate: Duration,

    /// Specify the rate of updating clients sessions
    #[serde(with = "humantime_serde")]
    pub clients_sessions_update_rate: Duration,

    /// If console logging is enabled, specify the interval at which that happens
    #[serde(with = "humantime_serde")]
    pub console_logging_update_interval: Duration,

    /// Specify the update rate of running stats for the legacy `/metrics/mixing` endpoint
    #[serde(with = "humantime_serde")]
    pub legacy_mixing_metrics_update_rate: Duration,
}

impl Debug {
    const DEFAULT_CONSOLE_LOGGING_INTERVAL: Duration = Duration::from_millis(60_000);
    const DEFAULT_LEGACY_MIXING_UPDATE_RATE: Duration = Duration::from_millis(30_000);
    const DEFAULT_AGGREGATOR_UPDATE_RATE: Duration = Duration::from_secs(5);
    const DEFAULT_STALE_MIXNET_METRICS_UPDATE_RATE: Duration = Duration::from_secs(3600);
    const DEFAULT_CLIENT_SESSIONS_UPDATE_RATE: Duration = Duration::from_secs(3600);
    const GLOBAL_PROMETHEUS_COUNTERS_UPDATE_INTERVAL: Duration = Duration::from_secs(30);
    const DEFAULT_PENDING_EGRESS_PACKETS_UPDATE_RATE: Duration = Duration::from_secs(30);
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            log_stats_to_console: true,
            console_logging_update_interval: Self::DEFAULT_CONSOLE_LOGGING_INTERVAL,
            legacy_mixing_metrics_update_rate: Self::DEFAULT_LEGACY_MIXING_UPDATE_RATE,
            aggregator_update_rate: Self::DEFAULT_AGGREGATOR_UPDATE_RATE,
            stale_mixnet_metrics_cleaner_rate: Self::DEFAULT_STALE_MIXNET_METRICS_UPDATE_RATE,
            global_prometheus_counters_update_rate:
                Self::GLOBAL_PROMETHEUS_COUNTERS_UPDATE_INTERVAL,
            pending_egress_packets_update_rate: Self::DEFAULT_PENDING_EGRESS_PACKETS_UPDATE_RATE,
            clients_sessions_update_rate: Self::DEFAULT_CLIENT_SESSIONS_UPDATE_RATE,
        }
    }
}
