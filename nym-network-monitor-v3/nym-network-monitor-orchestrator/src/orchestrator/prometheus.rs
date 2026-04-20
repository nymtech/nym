// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_metrics::{HistogramTimer, Metric, metrics_registry};
use std::sync::LazyLock;
use strum::{Display, EnumCount, EnumIter, EnumProperty, IntoEnumIterator};

pub static PROMETHEUS_METRICS: LazyLock<NetworkMonitorPrometheusMetrics> =
    LazyLock::new(NetworkMonitorPrometheusMetrics::initialise);

const TESTRUN_DURATION: &[f64] = &[
    // sub 5s (implicitly)
    5.,   // 5s - 15s
    15.,  // 15s - 30s
    30.,  // 30s - 40s
    40.,  // 40s - 45s
    45.,  // 45s - 50s
    50.,  // 50s - 55s
    55.,  // 55s - 60s
    60.,  // 1min - 2min
    120., // 2min - 5min
    300., // 5min+ (implicitly)
];

const NODE_LATENCY: &[f64] = &[
    // sub 1ms (implicitly)
    1.,    // 1ms - 5ms
    5.,    // 5ms - 10ms
    10.,   // 10ms - 20ms
    20.,   // 20ms - 50ms
    50.,   // 50ms - 100ms
    100.,  // 100ms - 200ms
    200.,  // 200ms - 500ms
    500.,  // 500ms - 1s
    1000., // 1s+ (implicitly)
];

const RECEIVED_PACKETS_RATIO: &[f64] = &[
    0.,   // 0 - 0.1
    0.1,  // 0.1 - 0.2
    0.2,  // 0.2 - 0.3
    0.3,  // 0.3 - 0.4
    0.4,  // 0.4 - 0.5
    0.5,  // 0.5 - 0.6
    0.6,  // 0.6 - 0.7
    0.7,  // 0.7 - 0.8
    0.8,  // 0.8 - 0.9
    0.9,  // 0.9 - 0.95
    0.95, // 0.95 - 0.98
    0.98, // 0.98 - 0.99
    0.99, // 0.99+ (implicitly)
];

const AVG_PACKET_RRT: &[f64] = &[
    // sub 1ms (implicitly)
    1.,    // 1ms - 5ms
    5.,    // 5ms - 10ms
    10.,   // 10ms - 20ms
    20.,   // 20ms - 50ms
    50.,   // 50ms - 100ms
    100.,  // 100ms - 200ms
    200.,  // 200ms - 500ms
    500.,  // 500ms - 1s
    1000., // 1s+ (implicitly)
];

#[derive(Clone, Debug, EnumIter, Display, EnumProperty, EnumCount, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case", prefix = "nym_network_monitor_")]
pub enum PrometheusMetric {
    #[strum(props(help = "The number of requests to assign a mix port to an agent"))]
    MixPortRequests,

    #[strum(props(help = "The number of failed requests to assign a mix port to an agent"))]
    MixPortRequestsFailures,

    #[strum(props(
        help = "The number of requests to announce an agent to the network monitors contract"
    ))]
    AgentAnnounceRequests,

    #[strum(props(
        help = "The number of requests to announce an agent that was either malformed or unknown to the orchestrator"
    ))]
    BadAgentAnnouncementRequests,

    #[strum(props(
        help = "The number of duplicate requests to announce an agent to the network monitors contract (agent has already been announced before)"
    ))]
    AgentDuplicateAnnouncementRequests,

    #[strum(props(
        help = "The number of successful announcements of an agent to the network monitors contract"
    ))]
    AgentContractAnnounceSuccesses,

    #[strum(props(
        help = "The number of failed announcements of an agent to the network monitors contract"
    ))]
    AgentContractAnnounceFailures,

    #[strum(props(help = "The number of requests to assign a test run to an agent"))]
    AgentTestrunRequests,

    #[strum(props(
        help = "The number of requests to assign a test run to an agent that was not known to the orchestrator"
    ))]
    AgentUnknownAgentTestrunRequests,

    #[strum(props(
        help = "The number of requests to assign a test run to an agent that was not announced to the network monitors contract"
    ))]
    AgentTestrunRequestsWithoutAnnouncement,

    #[strum(props(
        help = "The number of testrun requests that resulted in no work being assigned"
    ))]
    EmptyTestrunAssignments,

    #[strum(props(help = "The number of testrun requests that resulted in work being assigned"))]
    NonEmptyTestrunAssignments,

    #[strum(props(help = "The number of testrun results that were submitted by agents"))]
    TestRunResultSubmissions,

    #[strum(props(help = "The number of stale testruns that were evicted from the storage"))]
    StaleTestrunsEvicted,

    #[strum(props(
        help = "The number of testruns in progress that timed out and were evicted from the queue and the storage"
    ))]
    TimedOutTestrunsEvicted,

    #[strum(props(help = "The duration of a test run"))]
    TestDurationMs,

    #[strum(props(help = "The number of testruns that resulted in errors"))]
    TestrunsErrors,

    #[strum(props(help = "The approximate latency to a node in milliseconds"))]
    ApproximateNodeLatencyMs,

    #[strum(props(
        help = "Ratio of packets sent to packets received in a testrun (sent / received)"
    ))]
    TestrunReceivedPacketsRatio,

    #[strum(props(
        help = "The average time it took to receive a test packet back from a node under test"
    ))]
    AverageTestPacketRTTMs,

    #[strum(props(help = "The number of bonded nodes"))]
    BondedNymNodes,

    #[strum(props(
        help = "The number of successful Nym node data retrievals from self-described endpoints"
    ))]
    SuccessfulNymNodeDataRetrieval,

    #[strum(props(
        help = "The number of failed Nym node data retrievals from self-described endpoints"
    ))]
    FailedNymNodeDataRetrieval,
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
            PrometheusMetric::MixPortRequests => Metric::new_int_counter(&name, help),
            PrometheusMetric::MixPortRequestsFailures => Metric::new_int_counter(&name, help),
            PrometheusMetric::AgentAnnounceRequests => Metric::new_int_counter(&name, help),
            PrometheusMetric::BadAgentAnnouncementRequests => Metric::new_int_counter(&name, help),
            PrometheusMetric::AgentDuplicateAnnouncementRequests => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::AgentContractAnnounceSuccesses => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::AgentContractAnnounceFailures => Metric::new_int_counter(&name, help),
            PrometheusMetric::AgentTestrunRequests => Metric::new_int_counter(&name, help),
            PrometheusMetric::AgentUnknownAgentTestrunRequests => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::AgentTestrunRequestsWithoutAnnouncement => {
                Metric::new_int_counter(&name, help)
            }
            PrometheusMetric::EmptyTestrunAssignments => Metric::new_int_counter(&name, help),
            PrometheusMetric::NonEmptyTestrunAssignments => Metric::new_int_counter(&name, help),
            PrometheusMetric::TestRunResultSubmissions => Metric::new_int_counter(&name, help),
            PrometheusMetric::StaleTestrunsEvicted => Metric::new_int_counter(&name, help),
            PrometheusMetric::TimedOutTestrunsEvicted => Metric::new_int_counter(&name, help),
            PrometheusMetric::TestDurationMs => {
                Metric::new_histogram(&name, help, Some(TESTRUN_DURATION))
            }
            PrometheusMetric::TestrunsErrors => Metric::new_int_counter(&name, help),
            PrometheusMetric::ApproximateNodeLatencyMs => {
                Metric::new_histogram(&name, help, Some(NODE_LATENCY))
            }
            PrometheusMetric::TestrunReceivedPacketsRatio => {
                Metric::new_histogram(&name, help, Some(RECEIVED_PACKETS_RATIO))
            }
            PrometheusMetric::AverageTestPacketRTTMs => {
                Metric::new_histogram(&name, help, Some(AVG_PACKET_RRT))
            }
            PrometheusMetric::BondedNymNodes => Metric::new_int_gauge(&name, help),
            PrometheusMetric::SuccessfulNymNodeDataRetrieval => Metric::new_int_gauge(&name, help),
            PrometheusMetric::FailedNymNodeDataRetrieval => Metric::new_int_gauge(&name, help),
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

    pub fn metrics(&self) -> String {
        metrics_registry().to_string()
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
        assert_eq!(23, PrometheusMetric::COUNT)
    }

    #[test]
    fn every_variant_has_help_property() {
        for variant in PrometheusMetric::iter() {
            assert!(variant.get_str("help").is_some())
        }
    }
}
