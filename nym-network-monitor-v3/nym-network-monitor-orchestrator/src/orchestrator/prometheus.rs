// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_metrics::{HistogramTimer, Metric, metrics_registry};
use std::sync::LazyLock;
use strum::{Display, EnumCount, EnumIter, EnumProperty, IntoEnumIterator};

pub static PROMETHEUS_METRICS: LazyLock<NetworkMonitorPrometheusMetrics> =
    LazyLock::new(NetworkMonitorPrometheusMetrics::initialise);

#[derive(Clone, Debug, EnumIter, Display, EnumProperty, EnumCount, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case", prefix = "nym_network_monitor_")]
pub enum PrometheusMetric {
    #[strum(props(help = "placeholder for initial compilation"))]
    Placeholder,
}

impl PrometheusMetric {
    fn name(&self) -> String {
        self.to_string()
    }

    fn help(&self) -> &'static str {
        // SAFETY: every variant has a `help` prop defined (and there's a unit test is checking for that)
        #[allow(clippy::unwrap_used)]
        self.get_str("help").unwrap()
    }

    fn to_registrable_metric(&self) -> Option<Metric> {
        let name = self.name();
        let help = self.help();

        match self {
            _ => todo!(),
        }
    }

    fn set(&self, value: i64) {
        let reg = metrics_registry();
        if !reg.set(&self.name(), value)
            && let Some(registrable) = self.to_registrable_metric()
        {
            reg.register_metric(registrable);
            reg.set(&self.name(), value);
        }
    }

    fn set_float(&self, value: f64) {
        metrics_registry().set_float(&self.name(), value);
    }

    fn inc(&self) {
        metrics_registry().inc(&self.name());
    }

    fn inc_by(&self, value: i64) {
        metrics_registry().inc_by(&self.name(), value);
    }

    fn observe_histogram(&self, value: f64) {
        let reg = metrics_registry();
        if !reg.add_to_histogram(&self.name(), value)
            && let Some(registrable) = self.to_registrable_metric()
        {
            reg.register_metric(registrable);
            reg.add_to_histogram(&self.name(), value);
        }
    }

    fn start_timer(&self) -> Option<HistogramTimer> {
        metrics_registry().start_timer(&self.name())
    }
}

#[non_exhaustive]
pub struct NetworkMonitorPrometheusMetrics {}

impl NetworkMonitorPrometheusMetrics {
    // initialise all fields on startup with default values so that they'd be immediately available for query
    pub(crate) fn initialise() -> Self {
        let registry = metrics_registry();

        // we can't initialise complex metrics as their names will only be fully known at runtime
        for kind in PrometheusMetric::iter() {
            if let Some(metric) = kind.to_registrable_metric() {
                registry.register_metric(metric);
            }
        }

        NetworkMonitorPrometheusMetrics {}
    }

    pub fn set(&self, metric: PrometheusMetric, value: i64) {
        metric.set(value)
    }

    pub fn set_float(&self, metric: PrometheusMetric, value: f64) {
        metric.set_float(value)
    }

    pub fn inc(&self, metric: PrometheusMetric) {
        metric.inc()
    }

    pub fn inc_by(&self, metric: PrometheusMetric, value: i64) {
        metric.inc_by(value)
    }

    pub fn observe_histogram(&self, metric: PrometheusMetric, value: f64) {
        metric.observe_histogram(value)
    }

    pub fn start_timer(&self, metric: PrometheusMetric) -> Option<HistogramTimer> {
        metric.start_timer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn prometheus_metrics() {
        // a sanity check for anyone adding new metrics. if this test fails,
        // make sure any methods on `PrometheusMetric` enum don't need updating
        // or require custom Display impl
        assert_eq!(0, PrometheusMetric::COUNT)
    }

    #[test]
    fn every_variant_has_help_property() {
        for variant in PrometheusMetric::iter() {
            assert!(variant.get_str("help").is_some())
        }
    }
}
