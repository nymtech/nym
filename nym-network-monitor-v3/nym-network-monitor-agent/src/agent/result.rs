// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::egress::EgressConnectionStatistics;
use nym_network_monitor_orchestrator_requests::models::{
    ExercisedInterface, InterfaceMeasurement, TestKind,
};
use std::collections::BTreeMap;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::warn;

// TODO: once created, move this struct to a shared models library
/// What ONE of a node's interfaces returned: packets out, packets back, and the timing of what came
/// back.
///
/// Named for the SHAPE of the measurement rather than for a role or a phase, because it is one kind
/// of measurement rather than the only conceivable one: a future test measuring something that is not
/// a delivery ratio becomes a sibling of this, at which point these two sit behind one enum and
/// nothing that fills a `PacketDelivery` in has to change.
///
/// Fields are populated incrementally as a probe progresses; `None` means that step was never
/// reached rather than that it yielded nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct PacketDelivery {
    /// Duration of the Noise handshake on the ingress (responder) side, if completed.
    ///
    /// Both handshake figures stay empty on a leg with no Noise on it at all, such as one that
    /// forwards over a client websocket, where an absent figure is the honest report and not a gap.
    pub(crate) ingress_noise_handshake: Option<Duration>,

    /// Duration of the Noise handshake on the egress (initiator) side, if completed.
    pub(crate) egress_noise_handshake: Option<Duration>,

    /// Number of sphinx packets successfully sent to the node under test.
    pub(crate) packets_sent: usize,

    /// Number of sphinx packets returned by the node and successfully received.
    pub(crate) packets_received: usize,

    /// Round-trip time of the very first probe packet, sent in isolation before any load is applied.
    /// Because the node is idle at this point, this value approximates the baseline network latency
    /// to the node without any queuing or processing overhead from the test itself.
    /// `None` if the initial probe did not complete successfully.
    pub(crate) approximate_latency: Option<Duration>,

    /// RTT statistics computed over all received packets, or `None` if no packets were received.
    pub(crate) packets_statistics: Option<LatencyDistribution>,

    /// Latency distribution of individual batch send operations recorded during the load test.
    /// Reflects how long each batch took to flush to the OS socket, giving a rough measure of
    /// egress throughput. `None` if no batches were sent.
    pub(crate) sending_statistics: Option<LatencyDistribution>,

    /// Whether any packet was received with an ID that had already been seen against this interface.
    /// Duplicates should never occur under normal operation; their presence may indicate a
    /// misbehaving or malicious node replaying packets.
    pub(crate) received_duplicates: bool,

    /// Why this interface measured nothing, or less than it should have.
    ///
    /// Per interface so that one dead leg of a multi-interface run does not make its healthy legs
    /// unreportable. NOTE: this is currently LOCAL, for logging and tests only, because
    /// [`InterfaceMeasurement`] has no error field to carry it. Folding it into the run-level error
    /// instead would be wrong: that field drives `was_reachable`, so a gateway with a broken ingest
    /// and a working delivery would be reported as unreachable while scoring 0.5.
    pub(crate) error: Option<String>,
}

impl PacketDelivery {
    /// Calculates the percentage of packets received out of the total sent.
    pub(crate) fn received_percentage(&self) -> f64 {
        if self.packets_sent > 0 {
            (self.packets_received as f64 / self.packets_sent as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Records why this interface measured less than it should have.
    // the mixnode probe has one interface, so its failures are run-level; the per-interface error is
    // consumed by the gateway probe's phases
    #[allow(dead_code)]
    pub(crate) fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    /// Populates egress-side statistics from the finished [`EgressConnection`](crate::mixnet::egress::EgressConnection).
    /// Sets the egress Noise handshake duration and, if any batches were sent, the batch send
    /// latency distribution.
    pub(crate) fn set_egress_connection_statistics(&mut self, stats: EgressConnectionStatistics) {
        self.egress_noise_handshake = Some(stats.noise_handshake_duration);

        if !stats.packet_batches_sending_duration.is_empty() {
            self.sending_statistics = Some(LatencyDistribution::compute(
                &stats.packet_batches_sending_duration,
            ))
        }
    }

    /// Projects this interface's counts onto the measurement it is submitted as.
    fn into_measurement(
        self,
        interface: ExercisedInterface,
        sphinx_packet_delay: Duration,
    ) -> InterfaceMeasurement {
        InterfaceMeasurement {
            interface,
            ingress_noise_handshake: self.ingress_noise_handshake,
            egress_noise_handshake: self.egress_noise_handshake,
            sphinx_packet_delay,
            packets_sent: self.packets_sent,
            packets_received: self.packets_received,
            approximate_latency: self.approximate_latency,
            packets_statistics: self.packets_statistics.map(Into::into),
            sending_statistics: self.sending_statistics.map(Into::into),
            received_duplicates: self.received_duplicates,
        }
    }
}

/// Exactly the interfaces one probe is defined to exercise, each measured at most once.
///
/// The set is SEALED at construction from the probe's expected interfaces, which is what fixes the
/// denominator of the score computed downstream: an interface that produced nothing has to be
/// submitted as a zero rather than omitted, since an omission would shrink the average and let a
/// node whose second interface never ran tie with one that passed both.
///
/// Hence no `insert` and no `remove`. A probe may only fill in a slot its own expected set
/// established, an unexpected interface is unrepresentable, and a leg that never ran keeps its zeroed
/// seed. Ordered rather than hashed so a submitted payload's array order does not vary between runs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Measurements(BTreeMap<ExercisedInterface, PacketDelivery>);

impl Measurements {
    /// Seeds a zeroed measurement for every interface in `expected`.
    pub(crate) fn new(expected: &[ExercisedInterface]) -> Self {
        Measurements(
            expected
                .iter()
                .map(|&interface| (interface, PacketDelivery::default()))
                .collect(),
        )
    }

    /// Records what one interface measured.
    ///
    /// An interface outside the expected set is dropped with a warning rather than inserted, matching
    /// what the orchestrator does with one the role should not have produced. Reaching it means a
    /// probe measured something its own expected set does not name, which is a defect in the probe.
    pub(crate) fn record(&mut self, interface: ExercisedInterface, measured: PacketDelivery) {
        match self.0.get_mut(&interface) {
            Some(slot) => *slot = measured,
            None => warn!(
                "a probe measured {interface} which is not in its expected set, so it will not be reported"
            ),
        }
    }

    /// One interface's measurement, or `None` if this probe does not produce that interface.
    // read back by the gateway probe, which reports each phase as it completes
    #[allow(dead_code)]
    pub(crate) fn get(&self, interface: ExercisedInterface) -> Option<&PacketDelivery> {
        self.0.get(&interface)
    }
}

/// Captures the outcome of a single test run against one node: the run-level facts, plus one
/// measurement per interface the run was expected to exercise.
#[derive(Debug, Clone)]
pub(crate) struct TestRunResult {
    /// What this run measured, echoed onto the submission so the orchestrator records it under the
    /// kind it handed out. Carried on the result rather than supplied at conversion time, so a run
    /// cannot be submitted under a kind other than the one it was probed for.
    pub(crate) kind: TestKind,

    /// The timestamp when the test run was initiated.
    pub(crate) start_time: OffsetDateTime,

    /// The (constant) delay of the sphinx packet set during the test run. Run-level, because one run
    /// asks the same delay of the node on every leg, and echoed onto each measurement.
    pub(crate) sphinx_packet_delay: Duration,

    /// Why the whole run failed, which is the only thing that makes a node UNREACHABLE: the
    /// orchestrator reads `was_reachable` off this field's absence. A failure confined to one
    /// interface belongs on that interface instead.
    pub(crate) error: Option<String>,

    /// What each expected interface returned.
    pub(crate) measurements: Measurements,
}

impl TestRunResult {
    /// A run about to start: every expected interface seeded at zero, which is already a submittable
    /// result should nothing else happen.
    pub(crate) fn new(
        kind: TestKind,
        sphinx_packet_delay: Duration,
        expected: &[ExercisedInterface],
    ) -> Self {
        TestRunResult {
            kind,
            start_time: OffsetDateTime::now_utc(),
            sphinx_packet_delay,
            error: None,
            measurements: Measurements::new(expected),
        }
    }

    /// Records the run-level failure that stopped every interface.
    pub(crate) fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }
}

/// Latency statistics computed over the set of test packets received or sent during a stress test.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LatencyDistribution {
    /// Minimum latency duration it took to send or receive a test packet.
    pub minimum: Duration,

    /// Average latency duration it took to send or receive a test packet.
    pub mean: Duration,

    /// Median latency duration it took to send or receive a test packet.
    /// For an even number of samples, this is the arithmetic mean of the two middle values.
    pub median: Duration,

    /// Maximum latency duration it took to send or receive a test packet.
    pub maximum: Duration,

    /// The standard deviation of the latency duration it took to send or receive the test packets.
    pub standard_deviation: Duration,
}

impl LatencyDistribution {
    /// Computes statistics from a slice of per-packet RTT durations.
    /// Returns zeroed statistics if `raw_results` is empty.
    pub fn compute(raw_results: &[Duration]) -> Self {
        if raw_results.is_empty() {
            return LatencyDistribution {
                minimum: Duration::ZERO,
                mean: Duration::ZERO,
                median: Duration::ZERO,
                maximum: Duration::ZERO,
                standard_deviation: Duration::ZERO,
            };
        }

        let mut sorted = raw_results.to_vec();
        sorted.sort();

        let minimum = sorted[0];

        // SAFETY: we have ensured our list is not empty
        #[allow(clippy::unwrap_used)]
        let maximum = *sorted.last().unwrap();
        let median = Self::duration_median(&sorted);
        let mean = Self::duration_mean(&sorted);
        let standard_deviation = Self::duration_standard_deviation(&sorted, mean);

        LatencyDistribution {
            minimum,
            mean,
            median,
            maximum,
            standard_deviation,
        }
    }

    /// Computes the median of an already-sorted slice of durations.
    /// For an even count, returns the arithmetic mean of the two middle elements.
    /// Caller must ensure `sorted` is non-empty and ordered ascending.
    fn duration_median(sorted: &[Duration]) -> Duration {
        let len = sorted.len();
        let mid = len / 2;
        if len % 2 == 1 {
            sorted[mid]
        } else {
            (sorted[mid - 1] + sorted[mid]) / 2
        }
    }

    /// Computes the arithmetic mean of a slice of durations.
    /// Returns [`Duration::ZERO`] for an empty slice.
    fn duration_mean(data: &[Duration]) -> Duration {
        if data.is_empty() {
            return Default::default();
        }

        let sum = data.iter().sum::<Duration>();
        // packet counts realistically fit in a u32; a test sending 4 billion packets would
        // have other problems first
        let count = data.len() as u32;

        sum / count
    }

    /// Computes the population standard deviation (divides by N, not N-1) of the RTT durations.
    /// Precision is truncated to microseconds, which is sufficient for network latency.
    fn duration_standard_deviation(data: &[Duration], mean: Duration) -> Duration {
        if data.is_empty() {
            return Default::default();
        }

        let variance_micros = data
            .iter()
            .map(|&value| {
                let diff = mean.abs_diff(value);
                // truncate to microseconds — nanosecond precision is noise for network RTTs
                let diff_micros = diff.as_micros();
                diff_micros * diff_micros
            })
            .sum::<u128>()
            / data.len() as u128;

        // u128 easily holds squared microsecond values for any realistic RTT (< thousands of seconds)
        let std_deviation_micros = (variance_micros as f64).sqrt() as u64;
        Duration::from_micros(std_deviation_micros)
    }
}

impl From<LatencyDistribution>
    for nym_network_monitor_orchestrator_requests::models::LatencyDistribution
{
    fn from(value: LatencyDistribution) -> Self {
        Self {
            minimum: value.minimum,
            mean: value.mean,
            median: value.median,
            maximum: value.maximum,
            standard_deviation: value.standard_deviation,
        }
    }
}

/// Projects a finished run onto the submission shape: one measurement per interface it was expected
/// to exercise, in a stable order, whether or not that interface produced anything.
impl From<TestRunResult> for nym_network_monitor_orchestrator_requests::models::TestRunResult {
    fn from(value: TestRunResult) -> Self {
        let sphinx_packet_delay = value.sphinx_packet_delay;
        let measurements = value
            .measurements
            .0
            .into_iter()
            .map(|(interface, measured)| measured.into_measurement(interface, sphinx_packet_delay))
            .collect();

        Self {
            kind: value.kind,
            time_taken: (OffsetDateTime::now_utc() - value.start_time).unsigned_abs(),
            error: value.error,
            measurements,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use nym_network_monitor_orchestrator_requests::models as api;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn empty_slice_gives_zero_stats() {
        let stats = LatencyDistribution::compute(&[]);
        assert_eq!(stats.minimum, Duration::ZERO);
        assert_eq!(stats.maximum, Duration::ZERO);
        assert_eq!(stats.mean, Duration::ZERO);
        assert_eq!(stats.median, Duration::ZERO);
        assert_eq!(stats.standard_deviation, Duration::ZERO);
    }

    #[test]
    fn single_value_has_zero_deviation() {
        let stats = LatencyDistribution::compute(&[ms(42)]);
        assert_eq!(stats.minimum, ms(42));
        assert_eq!(stats.maximum, ms(42));
        assert_eq!(stats.mean, ms(42));
        assert_eq!(stats.median, ms(42));
        assert_eq!(stats.standard_deviation, Duration::ZERO);
    }

    #[test]
    fn two_equal_values_have_zero_deviation() {
        let stats = LatencyDistribution::compute(&[ms(10), ms(10)]);
        assert_eq!(stats.mean, ms(10));
        assert_eq!(stats.median, ms(10));
        assert_eq!(stats.standard_deviation, Duration::ZERO);
    }

    #[test]
    fn median_odd_count_picks_middle() {
        // sorted: 10, 20, 30, 40, 50 -> median = 30
        let data = [ms(40), ms(10), ms(50), ms(20), ms(30)];
        let stats = LatencyDistribution::compute(&data);
        assert_eq!(stats.median, ms(30));
    }

    #[test]
    fn median_even_count_averages_two_middle() {
        // sorted: 10, 20, 30, 40 -> median = (20 + 30) / 2 = 25
        let data = [ms(30), ms(10), ms(40), ms(20)];
        let stats = LatencyDistribution::compute(&data);
        assert_eq!(stats.median, ms(25));
    }

    #[test]
    fn min_max_are_correct() {
        let data = [ms(30), ms(10), ms(50), ms(20)];
        let stats = LatencyDistribution::compute(&data);
        assert_eq!(stats.minimum, ms(10));
        assert_eq!(stats.maximum, ms(50));
    }

    #[test]
    fn mean_is_correct() {
        // mean of 10, 20, 30, 40 = 25 ms
        let data = [ms(10), ms(20), ms(30), ms(40)];
        let stats = LatencyDistribution::compute(&data);
        assert_eq!(stats.mean, ms(25));
    }

    #[test]
    fn standard_deviation_known_values() {
        // population std-dev of {10, 20, 30, 40} ms:
        //   mean = 25, deviations = {-15, -5, 5, 15}
        //   variance = (225 + 25 + 25 + 225) / 4 = 125
        //   std-dev = sqrt(125) ≈ 11.180 ms → truncated to microseconds = 11180 µs
        let data = [ms(10), ms(20), ms(30), ms(40)];
        let stats = LatencyDistribution::compute(&data);
        let expected = Duration::from_micros(11180);
        // allow ±1 µs for floating-point rounding
        let diff = stats.standard_deviation.abs_diff(expected);
        assert!(
            diff <= Duration::from_micros(1),
            "std-dev {:.3?} not within 1µs of expected {:.3?}",
            stats.standard_deviation,
            expected
        );
    }

    fn mixnode() -> &'static [ExercisedInterface] {
        &[ExercisedInterface::MixForwarding]
    }

    fn gateway() -> &'static [ExercisedInterface] {
        &[
            ExercisedInterface::ClientIngest,
            ExercisedInterface::ClientDelivery,
        ]
    }

    fn measured(sent: usize, received: usize) -> PacketDelivery {
        PacketDelivery {
            packets_sent: sent,
            packets_received: received,
            ..Default::default()
        }
    }

    #[test]
    fn measurement_fields_survive_being_recorded() {
        let stats = LatencyDistribution::compute(&[ms(10), ms(20)]);
        let mut delivery = PacketDelivery {
            ingress_noise_handshake: Some(ms(5)),
            egress_noise_handshake: Some(ms(7)),
            packets_statistics: Some(stats),
            ..measured(100, 95)
        };
        delivery.set_error("timeout");

        let mut result = TestRunResult::new(TestKind::Stress, ms(2), mixnode());
        result
            .measurements
            .record(ExercisedInterface::MixForwarding, delivery);

        let recorded = result
            .measurements
            .get(ExercisedInterface::MixForwarding)
            .expect("the seeded interface went missing");
        assert_eq!(recorded.ingress_noise_handshake, Some(ms(5)));
        assert_eq!(recorded.egress_noise_handshake, Some(ms(7)));
        assert_eq!(recorded.packets_sent, 100);
        assert_eq!(recorded.packets_received, 95);
        assert_eq!(recorded.packets_statistics, Some(stats));
        assert_eq!(recorded.error.as_deref(), Some("timeout"));
    }

    // the denominator is fixed by the expected set, so a run that measured NOTHING still submits one
    // measurement per interface. an omission would be scored over a shorter set and would flatter a
    // node whose interface never answered
    #[test]
    fn an_unmeasured_run_still_submits_every_expected_interface() {
        let result = TestRunResult::new(TestKind::Liveness, ms(2), gateway());

        let wire: api::TestRunResult = result.into();
        let interfaces: Vec<_> = wire.measurements.iter().map(|m| m.interface).collect();
        assert_eq!(
            interfaces,
            vec![
                ExercisedInterface::ClientIngest,
                ExercisedInterface::ClientDelivery
            ]
        );
        assert!(wire.measurements.iter().all(|m| m.packets_sent == 0));
    }

    // one interface measuring nothing must not shrink the set the other is averaged against
    #[test]
    fn one_measured_interface_does_not_displace_its_unmeasured_sibling() {
        let mut result = TestRunResult::new(TestKind::Liveness, ms(2), gateway());
        result
            .measurements
            .record(ExercisedInterface::ClientIngest, measured(50, 50));

        let wire: api::TestRunResult = result.into();
        assert_eq!(wire.measurements.len(), 2);

        let ingest = wire
            .measurements
            .iter()
            .find(|m| m.interface == ExercisedInterface::ClientIngest)
            .expect("the measured interface is missing");
        assert_eq!(ingest.received_ratio(), 1.0);

        let delivery = wire
            .measurements
            .iter()
            .find(|m| m.interface == ExercisedInterface::ClientDelivery)
            .expect("the unmeasured interface was dropped");
        assert_eq!(delivery.received_ratio(), 0.0);
    }

    // the set is sealed at construction: a probe cannot add an interface it was not expected to
    // produce, since doing so would change the denominator the score is taken over
    #[test]
    fn an_unexpected_interface_is_not_recorded() {
        let mut result = TestRunResult::new(TestKind::Liveness, ms(2), mixnode());
        result
            .measurements
            .record(ExercisedInterface::ClientDelivery, measured(50, 50));

        assert!(
            result
                .measurements
                .get(ExercisedInterface::ClientDelivery)
                .is_none()
        );
        let wire: api::TestRunResult = result.into();
        assert_eq!(wire.measurements.len(), 1);
        assert_eq!(
            wire.measurements[0].interface,
            ExercisedInterface::MixForwarding
        );
    }

    // the run-level delay is what every measurement reports, since one run asks the same delay of the
    // node on each of its legs
    #[test]
    fn the_run_level_sphinx_delay_reaches_every_measurement() {
        let result = TestRunResult::new(TestKind::Liveness, ms(3), gateway());

        let wire: api::TestRunResult = result.into();
        assert!(
            wire.measurements
                .iter()
                .all(|m| m.sphinx_packet_delay == ms(3))
        );
    }
}
