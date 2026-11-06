// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_metrics::{Metric, metrics_registry};
use strum::{Display, EnumProperty};

/// Every series the driver publishes. A series is registered at its first report rather than at
/// startup, where its zero would read as a timestamp from 1970, operations taking no time or
/// nothing pending.
#[derive(Clone, Copy, Debug, Display, EnumProperty)]
#[strum(serialize_all = "snake_case", prefix = "nym_epoch_driver_")]
pub(crate) enum PrometheusMetric {
    /// Falls behind the current time when an ended epoch does not advance.
    #[strum(props(help = "The end of the epoch in progress, as the epoch loop last read it"))]
    CurrentEpochEndTimestampSeconds,

    #[strum(props(help = "How long the attempt that last completed an epoch's operations took"))]
    EpochOperationsDurationSeconds,

    /// Cleared at every epoch's end.
    #[strum(props(help = "The epoch events pending as the last reconciliation began"))]
    PendingEpochEvents,

    /// Built up over the interval and cleared only at its end.
    #[strum(props(help = "The interval events pending as the last reconciliation began"))]
    PendingIntervalEvents,
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
            | PrometheusMetric::EpochOperationsDurationSeconds
            | PrometheusMetric::PendingEpochEvents
            | PrometheusMetric::PendingIntervalEvents => {
                Metric::new_int_gauge(&self.name(), self.help())
            }
        }
    }

    pub(crate) fn report(&self, value: i64) {
        let registry = metrics_registry();
        if !registry.set(&self.name(), value)
            && let Some(registrable) = self.to_registrable_metric()
        {
            registry.register_metric(registrable);
            registry.set(&self.name(), value);
        }
    }
}
