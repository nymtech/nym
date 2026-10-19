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
//! A variant that differs from a sibling only by which kind, role or outcome it counts is ONE
//! family carrying that dimension as a label, declared by [`PrometheusMetric::labels`]. The label
//! value is always produced from a domain enum at this boundary, never spelled at a call site: the
//! registry reports a wrong arity as a log line rather than a compile error, so fixing arity in the
//! typed methods below is what keeps a mistake in this file.
//!
//! The registry is pre-populated in [`NetworkMonitorPrometheusMetrics::initialise`] so that every
//! series is present (with a zero value) from the very first scrape — this avoids dashboards and
//! alerts interpreting the first observation as a reset. A labelled family exposes no child until
//! one is touched, so each child is created there too.

use crate::storage::models::TestKind;
use nym_metrics::{Metric, metrics_registry};
use std::sync::LazyLock;
use strum::{Display, EnumCount, EnumIter, EnumProperty, IntoEnumIterator, IntoStaticStr};

/// Process-wide handle to the orchestrator's Prometheus metrics. Lazily initialised on first
/// access; the initialisation pre-registers every [`PrometheusMetric`] variant so that scrapes
/// observe a complete set of zeroed series even before any event has fired.
pub static PROMETHEUS_METRICS: LazyLock<NetworkMonitorPrometheusMetrics> =
    LazyLock::new(NetworkMonitorPrometheusMetrics::initialise);

/// The test kind a series is broken down by.
const KIND: &str = "kind";

/// The self-described role a bonded node was classified under.
const ROLE: &str = "role";

/// How one operation ended. Always paired with a separate total for the operation itself, so that
/// a breakdown is never summed together with the thing it breaks down.
const OUTCOME: &str = "outcome";

/// Histogram buckets (upper bounds, in seconds) for [`PrometheusMetric::TestDurationSeconds`].
///
/// One family carries one bucket set across all of its children, so this has to span a liveness
/// wave bounded by a 1 minute lease and a stress run bounded by a 5 minute one. Resolution is
/// densest in the first minute, because the decision this informs is whether a wave finishes
/// comfortably inside its lease, which is a question about seconds rather than about the tail.
const TESTRUN_DURATION: &[f64] = &[
    // sub 1s (implicitly)
    1.,   // 1s - 2.5s
    2.5,  // 2.5s - 5s
    5.,   // 5s - 10s
    10.,  // 10s - 20s
    20.,  // 20s - 30s
    30.,  // 30s - 45s
    45.,  // 45s - 60s (the liveness lease)
    60.,  // 1min - 1.5min
    90.,  // 1.5min - 2min
    120., // 2min - 3min
    180., // 3min - 5min
    300., // 5min+ (implicitly, past the stress lease)
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

/// Histogram buckets for the liveness wave-size series. Spans both configured caps (100 mixnode
/// targets, 50 gateway ones) with a dedicated `<= 1` bucket, because a wave that has collapsed to a
/// single target reads very differently from one that is merely short: it means the population due
/// for that pairing has run dry.
const LIVENESS_WAVE_SIZE: &[f64] = &[
    1.,   // 1 - 2
    2.,   // 2 - 5
    5.,   // 5 - 10
    10.,  // 10 - 20
    20.,  // 20 - 50
    50.,  // 50 - 100 (the gateway cap)
    100., // 100+ (implicitly, past the mixnode cap)
];

/// How a bonded node was classified from the roles it describes itself as serving. A node that
/// answered with no roles at all is as unusable as one that never answered, so both are `Unknown`.
#[derive(Clone, Copy, Debug, EnumIter, IntoStaticStr, Eq, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum NodeRole {
    Mixnode,
    Gateway,
    MixnodeAndGateway,
    Unknown,
}

/// How one agent announcement ended. The request itself is counted separately.
#[derive(Clone, Copy, Debug, EnumIter, IntoStaticStr, Eq, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum AnnounceOutcome {
    /// The agent was already announced, so the contract transaction was skipped.
    Duplicate,

    ContractSuccess,
    ContractFailure,

    /// Rejected before reaching the contract: the announced addresses were not a plain ipv4/ipv6
    /// pair.
    MalformedAddresses,
}

/// How one request for work ended, for the requests that yielded none. A request that was assigned
/// work is counted against its kind where that work is built.
#[derive(Clone, Copy, Debug, EnumIter, IntoStaticStr, Eq, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum TestrunRequestOutcome {
    UnknownAgent,
    NotAnnounced,
    NoWork,
}

/// What the nym-api did with one submitted result.
#[derive(Clone, Copy, Debug, EnumIter, IntoStaticStr, Eq, PartialEq)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum SubmissionOutcome {
    Accepted,

    /// Discarded because that measurement was already stored. Expected when a batch is retried; a
    /// persistently non-zero rate means measurements are being lost.
    Duplicate,

    /// Dropped in the nym-api's per-entry validation.
    Rejected,
}

/// Every Prometheus series emitted by the orchestrator. Each variant maps to exactly one metric
/// family and must carry a `help` strum property — this is verified by the
/// `every_variant_has_help_property` test.
///
/// The Prometheus metric name is derived from the variant name via strum: `serialize_all =
/// "snake_case"` + the `nym_network_monitor_` prefix. So `MixPortRequests` becomes
/// `nym_network_monitor_mix_port_requests`. The concrete metric kind (counter / gauge / histogram,
/// plus bucket bounds) is chosen in [`PrometheusMetric::to_registrable_metric`], and the dimension
/// a family is broken down by, if any, in [`PrometheusMetric::labels`].
#[derive(Clone, Debug, EnumIter, Display, EnumProperty, EnumCount, Eq, Hash, PartialEq)]
#[strum(serialize_all = "snake_case", prefix = "nym_network_monitor_")]
pub enum PrometheusMetric {
    #[strum(props(
        help = "The number of requests to announce an agent to the network monitors contract"
    ))]
    AgentAnnounceRequests,

    #[strum(props(
        help = "How agent announcements ended, by outcome: already announced, accepted or rejected by the contract, or rejected here for a malformed address pair"
    ))]
    AgentAnnounceOutcomeTotal,

    #[strum(props(
        help = "The number of announcements that changed the cached noise key, ipv6 address or ed25519 identity of an already known agent (its previous authorisation may be left stale in the contract)"
    ))]
    AgentDetailsChanged,

    #[strum(props(help = "The number of requests to assign a test run to an agent"))]
    AgentTestrunRequests,

    #[strum(props(
        help = "The requests for work that yielded none, by reason: the agent was unknown, it had not been announced, or nothing was due. A request that was assigned work is counted against its kind instead"
    ))]
    AgentTestrunRequestOutcomeTotal,

    #[strum(props(help = "The number of testrun results that were submitted by agents"))]
    TestRunResultSubmissions,

    #[strum(props(help = "The number of stale testruns that were evicted from the storage"))]
    StaleTestrunsEvicted,

    #[strum(props(help = "The duration of a test run, in seconds, by test kind"))]
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
        help = "The number of bonded nodes under each self-described role classification"
    ))]
    BondedNymNodes,

    #[strum(props(help = "The duration of a full bonded-node refresh cycle, in seconds"))]
    NodeRefreshCycleSeconds,

    #[strum(props(
        help = "The total number of test packets dispatched across all submitted testruns"
    ))]
    TestPacketsSent,

    #[strum(props(
        help = "The total number of test packets received back across all submitted testruns"
    ))]
    TestPacketsReceived,

    #[strum(props(
        help = "What the nym-api did with each submitted stress-test result, by outcome"
    ))]
    SubmittedResultsTotal,

    // Assignments, in-flight runs and lease expiries are each broken down by kind: every kind is
    // separate machinery against its own population, so an operator has to be able to see that
    // gateway liveness is flowing without inferring it from a total.
    #[strum(props(help = "The number of test runs assigned to agents, by test kind"))]
    TestrunAssignmentsTotal,

    #[strum(props(
        help = "The number of test runs currently in progress (rows in testrun_in_progress), by test kind. Sum across the label for the total"
    ))]
    TestrunsInFlight,

    #[strum(props(
        help = "The number of test runs whose lease expired before a result arrived, by test kind. A persistently non-zero value means that kind's lease is too short for the work it has to cover"
    ))]
    TestrunLeasesExpiredTotal,

    #[strum(props(
        help = "The number of targets in an assigned liveness wave, by test kind. A distribution sitting well below the configured wave size means the population is keeping up with the cadence rather than the wave being the constraint"
    ))]
    LivenessWaveSize,

    // Scheduling telemetry, refreshed from storage on the telemetry task's own tick. Together these
    // answer whether a kind has spare capacity and how much, in units that convert into a cadence.
    #[strum(props(
        help = "The nodes due for a test and not currently in flight, by test kind. The scheduler's own eligibility predicate counted rather than limited, so it cannot report a population an assignment would not draw from"
    ))]
    TestrunsPending,

    #[strum(props(
        help = "Accumulated seconds during which a kind had work pending or in flight, by test kind. rate() over it is that kind's utilisation at any window, so the averaging window stays a query-time choice and no tick between scrapes is lost"
    ))]
    WorkPendingSecondsTotal,

    #[strum(props(
        help = "The in-flight test runs whose lease has already expired but which the sweep has not yet released, by test kind. Counted within the in-flight total rather than deducted from it, since an expired lease still holds its node"
    ))]
    InFlightExpired,

    #[strum(props(
        help = "How long the longest-running dispatch has been outstanding, in seconds, by test kind; zero when a kind holds nothing in flight. The leading indicator of the condition in_flight_expired only reports once a lease has already been breached"
    ))]
    OldestInFlightAgeSeconds,

    #[strum(props(
        help = "How overdue the oldest node a kind has measured at least once is, in seconds, by test kind; zero when every measured node is within its cadence. The primary 'are we keeping up' signal. Nodes never measured are reported by never_tested instead, as they have no due time to be overdue against"
    ))]
    HeadLatenessSeconds,

    #[strum(props(
        help = "The nodes a kind could test but holds no result for, by test kind. Separate from head lateness because the assignment order puts never-tested nodes first, so folding them in would leave the overdue figure permanently undefined on a network that keeps gaining nodes"
    ))]
    NeverTested,

    #[strum(props(
        help = "The nodes a kind can test at all - bonded, described, and carrying that kind's role - by test kind. The scheduler's own denominator, which the bonded counts are not"
    ))]
    TestablePopulation,

    #[strum(props(
        help = "The assignment rate required to hold a kind's cadence, in assignments per second, by test kind: testable population over the configured staleness interval over the configured wave size. Divide by the observed work-request rate to size the fleet, which keeps working past saturation"
    ))]
    RequiredAssignmentRate,

    #[strum(props(
        help = "The bonded nodes absent from every kind's population, for want of a usable description. Coverage rather than capacity: work that can never exist never appears as backlog"
    ))]
    ExcludedNodes,

    #[strum(props(
        help = "The agents that have requested work within the configured liveness window. A drop against its own recent baseline is partial fleet loss, which no throughput signal reveals, because work is pulled and the survivors simply take more of it"
    ))]
    AgentsSeenRecently,

    #[strum(props(
        help = "The completed scheduling-telemetry ticks. Should this stop advancing, every gauge above goes stale while reporting a perfectly idle fleet, so alert on it"
    ))]
    MetricsTaskTicksTotal,

    // Performance contract submission. Alert when the last complete epoch stops advancing.
    #[strum(props(
        help = "The newest epoch with nothing left to submit to the performance contract. Not advancing for longer than an epoch means submissions are not landing"
    ))]
    PerformanceSubmissionLastCompleteEpoch,

    #[strum(props(
        help = "1 when the coverage guard omitted a kind from the last submitted epoch, as too few eligible nodes had a reachable aggregate. A gateway-only node whose kind is omitted submits nothing"
    ))]
    PerformanceSubmissionKindOmitted,
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

    /// The dimension this family is broken down by, or nothing for a plain series. Every family
    /// here carries at most one label, so the children of one are a list rather than a product.
    fn labels(&self) -> &'static [&'static str] {
        match self {
            PrometheusMetric::TestDurationSeconds
            | PrometheusMetric::TestrunAssignmentsTotal
            | PrometheusMetric::TestrunsInFlight
            | PrometheusMetric::TestrunLeasesExpiredTotal
            | PrometheusMetric::LivenessWaveSize
            | PrometheusMetric::PerformanceSubmissionKindOmitted
            | PrometheusMetric::TestrunsPending
            | PrometheusMetric::WorkPendingSecondsTotal
            | PrometheusMetric::InFlightExpired
            | PrometheusMetric::OldestInFlightAgeSeconds
            | PrometheusMetric::HeadLatenessSeconds
            | PrometheusMetric::NeverTested
            | PrometheusMetric::TestablePopulation
            | PrometheusMetric::RequiredAssignmentRate => &[KIND],
            PrometheusMetric::BondedNymNodes => &[ROLE],
            PrometheusMetric::AgentAnnounceOutcomeTotal
            | PrometheusMetric::AgentTestrunRequestOutcomeTotal
            | PrometheusMetric::SubmittedResultsTotal => &[OUTCOME],
            _ => &[],
        }
    }

    /// The children that must exist before anything is recorded, so that a family is exposed
    /// complete and zeroed from the first scrape rather than appearing one series at a time.
    ///
    /// Empty for a plain series, and for a family it must be non-empty: the two are kept in step by
    /// the `every_family_declares_its_children` test.
    fn initial_label_values(&self) -> Vec<&'static str> {
        match self {
            // a stress wave is one target by construction, so the wave-size family covers the two
            // liveness kinds only and must not pre-create a stress child
            PrometheusMetric::LivenessWaveSize => vec![
                TestKind::MixnodeLiveness.into(),
                TestKind::GatewayLiveness.into(),
            ],
            PrometheusMetric::TestDurationSeconds
            | PrometheusMetric::TestrunAssignmentsTotal
            | PrometheusMetric::TestrunsInFlight
            | PrometheusMetric::TestrunLeasesExpiredTotal
            | PrometheusMetric::PerformanceSubmissionKindOmitted
            | PrometheusMetric::TestrunsPending
            | PrometheusMetric::WorkPendingSecondsTotal
            | PrometheusMetric::InFlightExpired
            | PrometheusMetric::OldestInFlightAgeSeconds
            | PrometheusMetric::HeadLatenessSeconds
            | PrometheusMetric::NeverTested
            | PrometheusMetric::TestablePopulation
            | PrometheusMetric::RequiredAssignmentRate => {
                TestKind::iter().map(Into::into).collect()
            }
            PrometheusMetric::BondedNymNodes => NodeRole::iter().map(Into::into).collect(),
            PrometheusMetric::AgentAnnounceOutcomeTotal => {
                AnnounceOutcome::iter().map(Into::into).collect()
            }
            PrometheusMetric::AgentTestrunRequestOutcomeTotal => {
                TestrunRequestOutcome::iter().map(Into::into).collect()
            }
            PrometheusMetric::SubmittedResultsTotal => {
                SubmissionOutcome::iter().map(Into::into).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Builds the concrete [`Metric`] this variant should register as (counter / gauge / histogram
    /// with the right bucket bounds, as a labelled family where it declares labels). Called from
    /// [`NetworkMonitorPrometheusMetrics::initialise`] to pre-populate the registry, and from the
    /// `set` / `observe_histogram` fallback paths to lazily register a metric that somehow wasn't
    /// set up yet.
    fn to_registrable_metric(&self) -> Option<Metric> {
        let name = self.name();
        let help = self.help();
        let labels = self.labels();

        match self {
            PrometheusMetric::AgentAnnounceRequests
            | PrometheusMetric::AgentAnnounceOutcomeTotal
            | PrometheusMetric::AgentDetailsChanged
            | PrometheusMetric::AgentTestrunRequests
            | PrometheusMetric::AgentTestrunRequestOutcomeTotal
            | PrometheusMetric::TestRunResultSubmissions
            | PrometheusMetric::StaleTestrunsEvicted
            | PrometheusMetric::TestrunsErrors
            | PrometheusMetric::TestPacketsSent
            | PrometheusMetric::TestPacketsReceived
            | PrometheusMetric::SubmittedResultsTotal
            | PrometheusMetric::TestrunAssignmentsTotal
            | PrometheusMetric::TestrunLeasesExpiredTotal
            | PrometheusMetric::WorkPendingSecondsTotal
            | PrometheusMetric::MetricsTaskTicksTotal => {
                Metric::new_int_counter_with_labels(&name, help, labels)
            }
            PrometheusMetric::TestrunsInFlight
            | PrometheusMetric::BondedNymNodes
            | PrometheusMetric::PerformanceSubmissionLastCompleteEpoch
            | PrometheusMetric::PerformanceSubmissionKindOmitted
            | PrometheusMetric::TestrunsPending
            | PrometheusMetric::InFlightExpired
            | PrometheusMetric::OldestInFlightAgeSeconds
            | PrometheusMetric::HeadLatenessSeconds
            | PrometheusMetric::NeverTested
            | PrometheusMetric::TestablePopulation
            | PrometheusMetric::ExcludedNodes
            | PrometheusMetric::AgentsSeenRecently => {
                Metric::new_int_gauge_with_labels(&name, help, labels)
            }
            // a required rate is a small fraction of an assignment per second, so an integer gauge
            // would read zero for every realistic population
            PrometheusMetric::RequiredAssignmentRate => {
                Metric::new_float_gauge_with_labels(&name, help, labels)
            }
            PrometheusMetric::TestDurationSeconds => {
                Metric::new_histogram_with_labels(&name, help, Some(TESTRUN_DURATION), labels)
            }
            PrometheusMetric::ApproximateNodeLatencyMs => {
                Metric::new_histogram_with_labels(&name, help, Some(NODE_LATENCY), labels)
            }
            PrometheusMetric::TestrunReceivedPacketsRatio => {
                Metric::new_histogram_with_labels(&name, help, Some(RECEIVED_PACKETS_RATIO), labels)
            }
            PrometheusMetric::AverageTestPacketRTTMs => {
                Metric::new_histogram_with_labels(&name, help, Some(AVG_PACKET_RTT), labels)
            }
            PrometheusMetric::NodeRefreshCycleSeconds => {
                Metric::new_histogram_with_labels(&name, help, Some(NODE_REFRESH_CYCLE), labels)
            }
            PrometheusMetric::LivenessWaveSize => {
                Metric::new_histogram_with_labels(&name, help, Some(LIVENESS_WAVE_SIZE), labels)
            }
        }
    }

    /// Sets the gauge child addressed by `labels` to `value`. If the metric has not yet been
    /// registered (shouldn't happen after `initialise`, but we're defensive), falls back to
    /// registering it first and retrying.
    fn set(&self, value: i64, labels: &[&str]) {
        let reg = metrics_registry();
        if !reg.set_with_labels(&self.name(), value, labels)
            && let Some(registrable) = self.to_registrable_metric()
        {
            reg.register_metric(registrable);
            reg.set_with_labels(&self.name(), value, labels);
        }
    }

    /// Sets the float gauge child addressed by `labels`. Same register-on-miss fallback as
    /// [`Self::set`].
    fn set_float(&self, value: f64, labels: &[&str]) {
        let reg = metrics_registry();
        if !reg.set_float_with_labels(&self.name(), value, labels)
            && let Some(registrable) = self.to_registrable_metric()
        {
            reg.register_metric(registrable);
            reg.set_float_with_labels(&self.name(), value, labels);
        }
    }

    fn inc(&self, labels: &[&str]) {
        metrics_registry().inc_with_labels(&self.name(), labels);
    }

    fn inc_by(&self, value: i64, labels: &[&str]) {
        metrics_registry().inc_by_with_labels(&self.name(), value, labels);
    }

    /// Records `value` into the histogram child addressed by `labels`. Same register-on-miss
    /// fallback as [`Self::set`].
    fn observe_histogram(&self, value: f64, labels: &[&str]) {
        let reg = metrics_registry();
        if !reg.add_to_histogram_with_labels(&self.name(), value, labels)
            && let Some(registrable) = self.to_registrable_metric()
        {
            reg.register_metric(registrable);
            reg.add_to_histogram_with_labels(&self.name(), value, labels);
        }
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
    /// Pre-registers every [`PrometheusMetric`] variant in the shared registry, and every child of
    /// the ones that are families, so that the very first scrape after startup already returns the
    /// full set of series with zero values. Without this, series only appear after their first
    /// observation, which can make dashboards and alerting rules misbehave (missing series vs.
    /// zeroed series are not the same signal).
    pub(crate) fn initialise() -> Self {
        let registry = metrics_registry();

        for metric in PrometheusMetric::iter() {
            if let Some(registrable) = metric.to_registrable_metric() {
                registry.register_metric(registrable);
            }

            // a family exposes no children until one is touched, so pre-creating the metric is not
            // enough on its own
            let name = metric.name();
            for value in metric.initial_label_values() {
                registry.init_label_values(&name, &[value]);
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
        metric.set(value, &[])
    }

    pub fn inc(&self, metric: PrometheusMetric) {
        metric.inc(&[])
    }

    pub fn inc_by(&self, metric: PrometheusMetric, value: i64) {
        metric.inc_by(value, &[])
    }

    pub fn observe_histogram(&self, metric: PrometheusMetric, value: f64) {
        metric.observe_histogram(value, &[])
    }

    pub(crate) fn inc_for_kind(&self, metric: PrometheusMetric, kind: TestKind) {
        metric.inc(&[kind.into()])
    }

    pub(crate) fn inc_by_for_kind(&self, metric: PrometheusMetric, kind: TestKind, value: i64) {
        metric.inc_by(value, &[kind.into()])
    }

    pub(crate) fn set_for_kind(&self, metric: PrometheusMetric, kind: TestKind, value: i64) {
        metric.set(value, &[kind.into()])
    }

    pub(crate) fn set_float_for_kind(&self, metric: PrometheusMetric, kind: TestKind, value: f64) {
        metric.set_float(value, &[kind.into()])
    }

    pub(crate) fn observe_for_kind(&self, metric: PrometheusMetric, kind: TestKind, value: f64) {
        metric.observe_histogram(value, &[kind.into()])
    }

    pub(crate) fn set_for_role(&self, metric: PrometheusMetric, role: NodeRole, value: i64) {
        metric.set(value, &[role.into()])
    }

    pub(crate) fn inc_for_announce_outcome(
        &self,
        metric: PrometheusMetric,
        outcome: AnnounceOutcome,
    ) {
        metric.inc(&[outcome.into()])
    }

    pub(crate) fn inc_for_request_outcome(
        &self,
        metric: PrometheusMetric,
        outcome: TestrunRequestOutcome,
    ) {
        metric.inc(&[outcome.into()])
    }

    pub(crate) fn inc_by_for_submission_outcome(
        &self,
        metric: PrometheusMetric,
        outcome: SubmissionOutcome,
        value: i64,
    ) {
        metric.inc_by(value, &[outcome.into()])
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
        assert_eq!(34, PrometheusMetric::COUNT)
    }

    #[test]
    fn every_variant_has_help_property() {
        for variant in PrometheusMetric::iter() {
            assert!(variant.get_str("help").is_some())
        }
    }

    #[test]
    fn every_family_declares_its_children() {
        // the two are written as separate matches, so a family that gained a label but no label
        // space would register and then expose nothing at all until something was recorded
        for variant in PrometheusMetric::iter() {
            assert_eq!(
                !variant.labels().is_empty(),
                !variant.initial_label_values().is_empty(),
                "{variant} declares labels {:?} and children {:?}",
                variant.labels(),
                variant.initial_label_values()
            )
        }
    }

    #[test]
    fn every_kind_is_covered_by_the_kind_families() {
        // a kind added without a child here would be silently absent from the exposition until it
        // was first recorded against
        for variant in PrometheusMetric::iter().filter(|v| v.labels() == [KIND]) {
            let children = variant.initial_label_values();
            let expected = match variant {
                // a stress wave is one target by construction
                PrometheusMetric::LivenessWaveSize => 2,
                _ => TestKind::iter().count(),
            };
            assert_eq!(expected, children.len(), "{variant} covers {children:?}")
        }
    }
}
