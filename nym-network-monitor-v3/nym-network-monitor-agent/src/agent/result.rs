// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::egress_connection::EgressConnectionStatistics;
use std::time::Duration;

// TODO: once created, move this struct to a shared models library
/// Captures the outcome of a single [`run_stress_test`](super::NodeStressTester::run_stress_test) run.
///
/// Fields are populated incrementally as the test progresses; absent values (`None`) indicate
/// that the corresponding step was not reached or did not produce a result.
#[derive(Debug, Clone, Default)]
pub(crate) struct TestRunResult {
    /// Duration of the Noise handshake on the ingress (responder) side, if completed.
    pub(crate) ingress_noise_handshake: Option<Duration>,

    /// Duration of the Noise handshake on the egress (initiator) side, if completed.
    pub(crate) egress_noise_handshake: Option<Duration>,

    /// Number of sphinx packets successfully sent to the node under test.
    pub(crate) packets_sent: usize,

    /// Number of sphinx packets returned by the node and successfully received.
    pub(crate) packets_received: usize,

    /// Round-trip time of the very first probe packet, sent in isolation before any load is applied.
    /// Because the node is idle at this point, this value approximates the baseline network latency
    /// to the node without any queuing or processing overhead from the stress test itself.
    /// `None` if the initial probe did not complete successfully.
    pub(crate) approximate_latency: Option<Duration>,

    /// RTT statistics computed over all received packets, or `None` if no packets were received.
    pub(crate) packets_statistics: Option<LatencyDistribution>,

    /// Latency distribution of individual batch send operations recorded during the load test.
    /// Reflects how long each batch took to flush to the OS socket, giving a rough measure of
    /// egress throughput. `None` if no batches were sent.
    pub(crate) sending_statistics: Option<LatencyDistribution>,

    /// Whether any packet was received with an ID that had already been seen in this test run.
    /// Duplicates should never occur under normal operation; their presence may indicate a
    /// misbehaving or malicious node replaying packets.
    pub(crate) received_duplicates: bool,

    /// Human-readable description of the first error that caused the test to abort if any.
    pub(crate) error: Option<String>,
}

impl TestRunResult {
    pub(crate) fn new_empty() -> Self {
        Default::default()
    }

    /// Calculates the percentage of packets received out of the total sent.
    pub(crate) fn received_percentage(&self) -> f64 {
        if self.packets_sent > 0 {
            (self.packets_received as f64 / self.packets_sent as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Records the duration of the ingress Noise handshake.
    pub(crate) fn set_ingress_noise_handshake(&mut self, duration: Duration) {
        self.ingress_noise_handshake = Some(duration);
    }

    /// Records the duration of the egress Noise handshake.
    pub(crate) fn set_egress_noise_handshake(&mut self, duration: Duration) {
        self.egress_noise_handshake = Some(duration);
    }

    /// Records the RTT of the initial probe packet as the baseline latency estimate.
    pub(crate) fn set_approximate_latency(&mut self, rtt: Duration) {
        self.approximate_latency = Some(rtt);
    }

    /// Sets the number of packets that were sent during the stress test.
    pub(crate) fn set_packets_sent(&mut self, count: usize) {
        self.packets_sent = count;
    }

    /// Sets the number of packets that were received back from the node under test.
    pub(crate) fn set_packets_received(&mut self, count: usize) {
        self.packets_received = count;
    }

    /// Attaches pre-computed RTT statistics for the received packets.
    pub(crate) fn set_packets_statistics(&mut self, stats: LatencyDistribution) {
        self.packets_statistics = Some(stats);
    }

    /// Marks that at least one duplicate packet ID was observed during the test run.
    pub(crate) fn set_received_duplicates(&mut self) {
        self.received_duplicates = true;
    }

    /// Records an error message that caused the test run to abort.
    pub(crate) fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }

    /// Populates egress-side statistics from the finished [`EgressConnection`](crate::egress_connection::EgressConnection).
    /// Sets the egress Noise handshake duration and, if any batches were sent, the batch send
    /// latency distribution.
    pub(crate) fn set_egress_connection_statistics(&mut self, stats: EgressConnectionStatistics) {
        self.set_egress_noise_handshake(stats.noise_handshake_duration);

        if !stats.packet_batches_sending_duration.is_empty() {
            self.sending_statistics = Some(LatencyDistribution::compute(
                &stats.packet_batches_sending_duration,
            ))
        }
    }
}

/// Latency statistics computed over the set of test packets received or sent during a stress test.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LatencyDistribution {
    /// Minimum latency duration it took to send or receive a test packet.
    pub minimum: Duration,

    /// Average latency duration it took to send or receive a test packet.
    pub mean: Duration,

    /// Maximum latency duration it took to send or receive a test packet.
    pub maximum: Duration,

    /// The standard deviation of the latency duration it took to send or receive the test packets.
    pub standard_deviation: Duration,
}

impl LatencyDistribution {
    /// Computes statistics from a slice of per-packet RTT durations.
    /// Returns zeroed statistics if `raw_results` is empty.
    pub fn compute(raw_results: &[Duration]) -> Self {
        let minimum = raw_results.iter().min().copied().unwrap_or_default();
        let maximum = raw_results.iter().max().copied().unwrap_or_default();

        let mean = Self::duration_mean(raw_results);
        let standard_deviation = Self::duration_standard_deviation(raw_results, mean);

        LatencyDistribution {
            minimum,
            mean,
            maximum,
            standard_deviation,
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
            maximum: value.maximum,
            standard_deviation: value.standard_deviation,
        }
    }
}

impl From<TestRunResult> for nym_network_monitor_orchestrator_requests::models::TestRunResult {
    fn from(value: TestRunResult) -> Self {
        Self {
            ingress_noise_handshake: value.ingress_noise_handshake,
            egress_noise_handshake: value.egress_noise_handshake,
            packets_sent: value.packets_sent,
            packets_received: value.packets_received,
            approximate_latency: value.approximate_latency,
            packets_statistics: value.packets_statistics.map(Into::into),
            sending_statistics: value.sending_statistics.map(Into::into),
            received_duplicates: value.received_duplicates,
            error: value.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn empty_slice_gives_zero_stats() {
        let stats = LatencyDistribution::compute(&[]);
        assert_eq!(stats.minimum, Duration::ZERO);
        assert_eq!(stats.maximum, Duration::ZERO);
        assert_eq!(stats.mean, Duration::ZERO);
        assert_eq!(stats.standard_deviation, Duration::ZERO);
    }

    #[test]
    fn single_value_has_zero_deviation() {
        let stats = LatencyDistribution::compute(&[ms(42)]);
        assert_eq!(stats.minimum, ms(42));
        assert_eq!(stats.maximum, ms(42));
        assert_eq!(stats.mean, ms(42));
        assert_eq!(stats.standard_deviation, Duration::ZERO);
    }

    #[test]
    fn two_equal_values_have_zero_deviation() {
        let stats = LatencyDistribution::compute(&[ms(10), ms(10)]);
        assert_eq!(stats.mean, ms(10));
        assert_eq!(stats.standard_deviation, Duration::ZERO);
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
    fn result_setters_populate_fields() {
        let mut result = TestRunResult::new_empty();
        result.set_ingress_noise_handshake(ms(5));
        result.set_egress_noise_handshake(ms(7));
        result.set_packets_sent(100);
        result.set_packets_received(95);
        result.set_error("timeout");

        let stats = LatencyDistribution::compute(&[ms(10), ms(20)]);
        result.set_packets_statistics(stats);

        assert_eq!(result.ingress_noise_handshake, Some(ms(5)));
        assert_eq!(result.egress_noise_handshake, Some(ms(7)));
        assert_eq!(result.packets_sent, 100);
        assert_eq!(result.packets_received, 95);
        assert_eq!(result.packets_statistics, Some(stats));
        assert_eq!(result.error.as_deref(), Some("timeout"));
    }
}
