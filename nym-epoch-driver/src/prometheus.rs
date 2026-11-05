// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_metrics::{Metric, metrics_registry};
use strum::{Display, EnumProperty};
use time::OffsetDateTime;

/// Every series the driver publishes: Unix timestamps, alerted on as `time() - x`. A series is
/// registered at its first set rather than at startup, where its zero would read as 1970.
#[derive(Clone, Copy, Debug, Display, EnumProperty)]
#[strum(serialize_all = "snake_case", prefix = "nym_epoch_driver_")]
pub(crate) enum PrometheusMetric {
    /// Falls behind the current time when an ended epoch does not advance.
    #[strum(props(help = "The end of the epoch in progress, as the epoch loop last read it"))]
    CurrentEpochEndTimestampSeconds,

    /// Falls behind when the loop hangs while the http api still answers.
    #[strum(props(help = "The time of the epoch loop's last interval read"))]
    EpochLoopTimestampSeconds,

    /// Falls behind when selection would run on an old view, which nothing on chain shows.
    #[strum(props(help = "The time of the last successful network view production"))]
    NetworkViewTimestampSeconds,
}

impl PrometheusMetric {
    fn name(&self) -> String {
        self.to_string()
    }

    fn help(&self) -> &'static str {
        self.get_str("help").unwrap_or_default()
    }

    fn to_registrable_metric(&self) -> Option<Metric> {
        match self {
            PrometheusMetric::CurrentEpochEndTimestampSeconds
            | PrometheusMetric::EpochLoopTimestampSeconds
            | PrometheusMetric::NetworkViewTimestampSeconds => {
                Metric::new_int_gauge(&self.name(), self.help())
            }
        }
    }

    pub(crate) fn report_timestamp(&self, at: OffsetDateTime) {
        let registry = metrics_registry();
        let value = at.unix_timestamp();
        if !registry.set(&self.name(), value)
            && let Some(registrable) = self.to_registrable_metric()
        {
            registry.register_metric(registrable);
            registry.set(&self.name(), value);
        }
    }
}
