// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_monitor_orchestrator_requests::models::{
    InterfaceMeasurement, RunMeasurements, TestKind,
};
use std::time::Duration;
use time::OffsetDateTime;

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

    /// Projects this interface's counts onto the measurement it is submitted as.
    fn into_measurement(self, sphinx_packet_delay: Duration) -> InterfaceMeasurement {
        InterfaceMeasurement {
            ingress_noise_handshake: self.ingress_noise_handshake,
            egress_noise_handshake: self.egress_noise_handshake,
            sphinx_packet_delay,
            packets_sent: self.packets_sent,
            packets_received: self.packets_received,
            approximate_latency: self.approximate_latency,
            packets_statistics: self.packets_statistics.map(Into::into),
            received_duplicates: self.received_duplicates,
        }
    }
}

/// What one run measured, shaped by its kind exactly as [`RunMeasurements`] is on the wire: one
/// delivery per interface the kind exercises.
///
/// Every slot exists from the moment the run starts, which is what fixes the denominator of the score
/// computed downstream: an interface that produced nothing is submitted as a zero rather than left
/// out, since an omission would shrink the average and let a node whose second interface never ran
/// tie with one that passed both. An interface the kind does not exercise is unrepresentable.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProbeMeasurements {
    MixnodeLiveness {
        mix_forwarding: PacketDelivery,
    },
    GatewayLiveness {
        client_ingest: PacketDelivery,
        client_delivery: PacketDelivery,
    },
    MixnodeStress {
        mix_forwarding: PacketDelivery,
    },
}

impl ProbeMeasurements {
    /// Every interface `kind` exercises, with nothing measured yet: what a run reports should
    /// nothing else happen.
    fn unmeasured(kind: TestKind) -> Self {
        match kind {
            TestKind::MixnodeLiveness => ProbeMeasurements::MixnodeLiveness {
                mix_forwarding: PacketDelivery::default(),
            },
            TestKind::GatewayLiveness => ProbeMeasurements::GatewayLiveness {
                client_ingest: PacketDelivery::default(),
                client_delivery: PacketDelivery::default(),
            },
            TestKind::MixnodeStress => ProbeMeasurements::MixnodeStress {
                mix_forwarding: PacketDelivery::default(),
            },
        }
    }

    /// Projects every delivery onto the measurement it is submitted as.
    fn into_wire(self, sphinx_packet_delay: Duration) -> RunMeasurements {
        match self {
            ProbeMeasurements::MixnodeLiveness { mix_forwarding } => {
                RunMeasurements::MixnodeLiveness {
                    mix_forwarding: mix_forwarding.into_measurement(sphinx_packet_delay),
                }
            }
            ProbeMeasurements::GatewayLiveness {
                client_ingest,
                client_delivery,
            } => RunMeasurements::GatewayLiveness {
                client_ingest: client_ingest.into_measurement(sphinx_packet_delay),
                client_delivery: client_delivery.into_measurement(sphinx_packet_delay),
            },
            ProbeMeasurements::MixnodeStress { mix_forwarding } => RunMeasurements::MixnodeStress {
                mix_forwarding: mix_forwarding.into_measurement(sphinx_packet_delay),
            },
        }
    }
}

/// Captures the outcome of a single test run against one node: the run-level facts, plus the
/// measurements its kind defines.
#[derive(Debug, Clone)]
pub(crate) struct TestRunResult {
    /// The timestamp when the test run was initiated.
    pub(crate) start_time: OffsetDateTime,

    /// The (constant) delay of the sphinx packet set during the test run. Run-level, because one run
    /// asks the same delay of the node on every leg, and echoed onto each measurement.
    pub(crate) sphinx_packet_delay: Duration,

    /// Why the whole run failed, which is the only thing that makes a node UNREACHABLE: the
    /// orchestrator reads `was_reachable` off this field's absence. A failure confined to one
    /// interface belongs on that interface instead.
    pub(crate) error: Option<String>,

    /// What each interface of the run's kind returned, which also says which kind it was, so a run
    /// cannot be submitted under a kind other than the one it was probed for.
    pub(crate) measurements: ProbeMeasurements,
}

impl TestRunResult {
    /// A run of `kind` about to start: every interface it exercises seeded at zero, which is already
    /// a submittable result should nothing else happen.
    pub(crate) fn new(kind: TestKind, sphinx_packet_delay: Duration) -> Self {
        TestRunResult {
            start_time: OffsetDateTime::now_utc(),
            sphinx_packet_delay,
            error: None,
            measurements: ProbeMeasurements::unmeasured(kind),
        }
    }

    /// Records the run-level failure that stopped every interface.
    pub(crate) fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }
}

/// Round-trip time statistics computed over the test packets received during a run.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LatencyDistribution {
    /// Minimum round-trip time of a test packet.
    pub minimum: Duration,

    /// Average round-trip time of a test packet.
    pub mean: Duration,

    /// Median round-trip time of a test packet.
    /// For an even number of samples, this is the arithmetic mean of the two middle values.
    pub median: Duration,

    /// Maximum round-trip time of a test packet.
    pub maximum: Duration,

    /// The standard deviation of the test packets' round-trip times.
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

/// Projects a finished run onto the submission shape: one measurement per interface its kind
/// exercises, whether or not that interface produced anything.
impl From<TestRunResult> for nym_network_monitor_orchestrator_requests::models::TestRunResult {
    fn from(value: TestRunResult) -> Self {
        Self {
            time_taken: (OffsetDateTime::now_utc() - value.start_time).unsigned_abs(),
            error: value.error,
            measurements: value.measurements.into_wire(value.sphinx_packet_delay),
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

    #[test]
    fn measurement_fields_survive_projection_onto_the_wire() {
        let stats = LatencyDistribution::compute(&[ms(10), ms(20)]);
        let mut result = TestRunResult::new(TestKind::MixnodeStress, ms(2));
        result.measurements = ProbeMeasurements::MixnodeStress {
            mix_forwarding: PacketDelivery {
                ingress_noise_handshake: Some(ms(5)),
                egress_noise_handshake: Some(ms(7)),
                packets_sent: 100,
                packets_received: 95,
                packets_statistics: Some(stats),
                ..Default::default()
            },
        };

        let wire: api::TestRunResult = result.into();
        let api::RunMeasurements::MixnodeStress {
            mix_forwarding: projected,
        } = wire.measurements
        else {
            panic!("projected onto the wrong kind: {wire:#?}");
        };
        assert_eq!(projected.ingress_noise_handshake, Some(ms(5)));
        assert_eq!(projected.egress_noise_handshake, Some(ms(7)));
        assert_eq!(projected.packets_sent, 100);
        assert_eq!(projected.packets_received, 95);
        assert_eq!(projected.packets_statistics, Some(stats.into()));
    }

    // each kind is seeded with its own shape, so a run that measured NOTHING is still submitted as
    // its kind with every interface present and zeroed. swapped arms would submit a stress run as
    // liveness, or a gateway run as a mixnode one
    #[test]
    fn every_kind_is_seeded_with_its_own_shape() {
        for kind in [
            TestKind::MixnodeLiveness,
            TestKind::GatewayLiveness,
            TestKind::MixnodeStress,
        ] {
            let wire: api::TestRunResult = TestRunResult::new(kind, ms(2)).into();

            assert_eq!(wire.kind(), kind);
            assert!(wire.measurements.all().iter().all(|m| m.packets_sent == 0));
        }
    }

    // the run-level delay is what every measurement reports, since one run asks the same delay of the
    // node on each of its legs
    #[test]
    fn the_run_level_sphinx_delay_reaches_every_measurement() {
        let wire: api::TestRunResult = TestRunResult::new(TestKind::GatewayLiveness, ms(3)).into();

        assert!(
            wire.measurements
                .all()
                .iter()
                .all(|m| m.sphinx_packet_delay == ms(3))
        );
    }
}
