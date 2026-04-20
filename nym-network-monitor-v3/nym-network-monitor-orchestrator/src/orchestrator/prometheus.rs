// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Prometheus metrics exposed by the orchestrator.
//!
//! Every series this service emits is declared up-front as a variant of [`PrometheusMetric`].
//! Each variant carries its Prometheus help string via strum's `EnumProperty` attribute and is
//! mapped to a concrete counter / gauge / histogram in [`PrometheusMetric::to_registrable_metric`].
//! Call sites emit values through the process-wide [`PROMETHEUS_METRICS`] handle, which forwards
//! to the underlying [`nym_metrics`] registry.
//!
//! The registry is pre-populated in [`NetworkMonitorPrometheusMetrics::initialise`] so that every
//! series is present (with a zero value) from the very first scrape — this avoids dashboards and
//! alerts interpreting the first observation as a reset.

use nym_metrics::{HistogramTimer, Metric, metrics_registry};
use std::sync::LazyLock;
use strum::{Display, EnumCount, EnumIter, EnumProperty, IntoEnumIterator};

/// Process-wide handle to the orchestrator's Prometheus metrics. Lazily initialised on first
/// access; the initialisation pre-registers every [`PrometheusMetric`] variant so that scrapes
/// observe a complete set of zeroed series even before any event has fired.
pub static PROMETHEUS_METRICS: LazyLock<NetworkMonitorPrometheusMetrics> =
    LazyLock::new(NetworkMonitorPrometheusMetrics::initialise);

/// Histogram buckets (upper bounds, in seconds) for [`PrometheusMetric::TestDurationSeconds`].
/// Densely spaced in the 40–60 s range because most completed runs cluster there and small
/// shifts in that band are the most interesting signal.
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

/// Histogram buckets (upper bounds, in milliseconds) for
/// [`PrometheusMetric::ApproximateNodeLatencyMs`]. Log-ish spacing from 1 ms up to 1 s — typical
/// mixnet latencies are well under 500 ms and anything past 1 s lands in the overflow bucket.
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

/// Histogram buckets for [`PrometheusMetric::TestrunReceivedPacketsRatio`] — `received / sent`,
/// so values live in `[0, 1]`. The dedicated `<= 0.0` bucket isolates the "got nothing" case from
/// "got a few", which otherwise would all collapse into a single low bucket; upper buckets are
/// dense near 1.0 because the difference between 99% and 95% delivery is operationally significant.
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

/// Histogram buckets (upper bounds, in seconds) for
/// [`PrometheusMetric::NodeRefreshCycleSeconds`]. Shape targets the expected range of a single
/// refresh sweep: under a minute when everything's healthy, up to ~10 min in degraded cases
/// (large topology × per-node timeouts × limited concurrency).
const NODE_REFRESH_CYCLE: &[f64] = &[
    // sub 1s (implicitly)
    1.,   // 1s - 5s
    5.,   // 5s - 10s
    10.,  // 10s - 30s
    30.,  // 30s - 60s
    60.,  // 1min - 2min
    120., // 2min - 5min
    300., // 5min - 10min
    600., // 10min+ (implicitly)
];

/// Histogram buckets (upper bounds, in milliseconds) for
/// [`PrometheusMetric::AverageTestPacketRTTMs`]. Same shape as [`NODE_LATENCY`] — this is the
/// mean per-packet round trip over a single testrun, not the approximation used for node latency.
const AVG_PACKET_RTT: &[f64] = &[
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

/// Every Prometheus series emitted by the orchestrator. Each variant maps to exactly one metric
/// and must carry a `help` strum property — this is verified by the `every_variant_has_help_property`
/// test.
///
/// The Prometheus metric name is derived from the variant name via strum: `serialize_all =
/// "snake_case"` + the `nym_network_monitor_` prefix. So `MixPortRequests` becomes
/// `nym_network_monitor_mix_port_requests`. The concrete metric kind (counter / gauge / histogram,
/// plus bucket bounds) is chosen in [`PrometheusMetric::to_registrable_metric`].
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

    #[strum(props(help = "The duration of a test run, in seconds"))]
    TestDurationSeconds,

    #[strum(props(help = "The number of testruns that resulted in errors"))]
    TestrunsErrors,

    #[strum(props(help = "The approximate latency to a node in milliseconds"))]
    ApproximateNodeLatencyMs,

    #[strum(props(
        help = "Ratio of packets sent to packets received in a testrun (received / sent)"
    ))]
    TestrunReceivedPacketsRatio,

    #[strum(props(
        help = "The average time it took to receive a test packet back from a node under test"
    ))]
    AverageTestPacketRTTMs,

    #[strum(props(
        help = "The number of bonded nodes classified as mixnode-only from their self-described roles"
    ))]
    BondedMixnodeNymNodes,

    #[strum(props(
        help = "The number of bonded nodes classified as gateway-only from their self-described roles"
    ))]
    BondedGatewayNymNodes,

    #[strum(props(help = "The number of bonded nodes advertising both mixnode and gateway roles"))]
    BondedMixnodeAndGatewayNymNodes,

    #[strum(props(
        help = "The number of bonded nodes whose self-described role could not be determined (unreachable or no roles reported)"
    ))]
    BondedUnknownNymNodes,

    #[strum(props(
        help = "The number of successful Nym node data retrievals from self-described endpoints"
    ))]
    SuccessfulNymNodeDataRetrieval,

    #[strum(props(
        help = "The number of failed Nym node data retrievals from self-described endpoints"
    ))]
    FailedNymNodeDataRetrieval,

    #[strum(props(help = "The duration of a full bonded-node refresh cycle, in seconds"))]
    NodeRefreshCycleSeconds,

    #[strum(props(
        help = "The number of test runs currently in progress (rows in testrun_in_progress)"
    ))]
    TestrunsInProgress,

    #[strum(props(help = "The total number of agents known to the orchestrator"))]
    KnownAgentsTotal,

    #[strum(props(
        help = "The number of known agents that have been announced to the network monitors contract"
    ))]
    KnownAgentsAnnounced,

    #[strum(props(
        help = "The total number of test packets dispatched across all submitted testruns"
    ))]
    TestPacketsSent,

    #[strum(props(
        help = "The total number of test packets received back across all submitted testruns"
    ))]
    TestPacketsReceived,
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

    /// Builds the concrete [`Metric`] this variant should register as (counter / gauge / histogram
    /// with the right bucket bounds). Called from [`NetworkMonitorPrometheusMetrics::initialise`]
    /// to pre-populate the registry, and from the `set` / `observe_histogram` fallback paths to
    /// lazily register a metric that somehow wasn't set up yet.
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
            PrometheusMetric::TestDurationSeconds => {
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
                Metric::new_histogram(&name, help, Some(AVG_PACKET_RTT))
            }
            PrometheusMetric::BondedMixnodeNymNodes => Metric::new_int_gauge(&name, help),
            PrometheusMetric::BondedGatewayNymNodes => Metric::new_int_gauge(&name, help),
            PrometheusMetric::BondedMixnodeAndGatewayNymNodes => Metric::new_int_gauge(&name, help),
            PrometheusMetric::BondedUnknownNymNodes => Metric::new_int_gauge(&name, help),
            PrometheusMetric::SuccessfulNymNodeDataRetrieval => Metric::new_int_gauge(&name, help),
            PrometheusMetric::FailedNymNodeDataRetrieval => Metric::new_int_gauge(&name, help),
            PrometheusMetric::NodeRefreshCycleSeconds => {
                Metric::new_histogram(&name, help, Some(NODE_REFRESH_CYCLE))
            }
            PrometheusMetric::TestrunsInProgress => Metric::new_int_gauge(&name, help),
            PrometheusMetric::KnownAgentsTotal => Metric::new_int_gauge(&name, help),
            PrometheusMetric::KnownAgentsAnnounced => Metric::new_int_gauge(&name, help),
            PrometheusMetric::TestPacketsSent => Metric::new_int_counter(&name, help),
            PrometheusMetric::TestPacketsReceived => Metric::new_int_counter(&name, help),
        }
    }

    /// Sets the gauge to `value`. If the metric has not yet been registered (shouldn't happen after
    /// `initialise`, but we're defensive), falls back to registering it first and retrying.
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

    /// Records `value` into the histogram. Same register-on-miss fallback as [`Self::set`].
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

/// Orchestrator-side handle to the process-wide Prometheus registry. Constructed once via
/// [`Self::initialise`] (held in the [`PROMETHEUS_METRICS`] static) and used from call sites to
/// emit values against the [`PrometheusMetric`] enum. All mutating methods are thin wrappers
/// around the corresponding methods on [`PrometheusMetric`].
#[non_exhaustive]
pub struct NetworkMonitorPrometheusMetrics {
    _private: (),
}

impl NetworkMonitorPrometheusMetrics {
    /// Pre-registers every [`PrometheusMetric`] variant in the shared registry so that the very
    /// first scrape after startup already returns the full set of series with zero values.
    /// Without this, series only appear after their first observation, which can make dashboards
    /// and alerting rules misbehave (missing series vs. zeroed series are not the same signal).
    pub(crate) fn initialise() -> Self {
        let registry = metrics_registry();

        // we can't initialise complex metrics as their names will only be fully known at runtime
        for kind in PrometheusMetric::iter() {
            if let Some(metric) = kind.to_registrable_metric() {
                registry.register_metric(metric);
            }
        }

        NetworkMonitorPrometheusMetrics { _private: () }
    }

    /// Renders the full registry in the Prometheus text exposition format — this is what the
    /// `/v1/metrics/prometheus` scrape endpoint returns.
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
        assert_eq!(32, PrometheusMetric::COUNT)
    }

    #[test]
    fn every_variant_has_help_property() {
        for variant in PrometheusMetric::iter() {
            assert!(variant.get_str("help").is_some())
        }
    }
}
