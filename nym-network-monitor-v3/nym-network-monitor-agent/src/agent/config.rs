// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_monitor_orchestrator_requests::models::AgentMixAddresses;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::time::Duration;

/// How many batches a liveness probe spreads its packets over, which is what makes it paced rather
/// than bursted. Chosen as a count of dispatches rather than a batch size so that the pacing
/// survives the profile's provisional packet count and rate being retuned: at the profile's own
/// values this is five dispatches of ten packets, 200ms apart, across a one second send window.
const LIVENESS_DISPATCHES_PER_PROBE: usize = 5;

/// The sending knobs of ONE test kind's probe.
///
/// The two kinds specify themselves from opposite ends, which is why this is built through
/// constructors rather than by setting fields: a stress test is a RATE held for a DURATION with the
/// packet count falling out of the two, while a liveness probe is a per-target COUNT whose send
/// window falls out of it. Everything downstream reads the resolved values and does not care which
/// end they came from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProbeProfile {
    /// Total number of packets the probe intends to send to one target. Reported as the sent count
    /// on success regardless of how many were actually pushed, so a node that applies back-pressure
    /// is penalised rather than flattered.
    pub(crate) expected_packets: usize,

    /// Target rate of packets (per second) sent to ONE target. Deliberately independent of how many
    /// targets a wave carries, so that a node's delivery ratio is comparable between a narrow wave
    /// and a full one.
    pub(crate) target_rate: usize,

    /// How long the probe spends sending, i.e. `expected_packets / target_rate`.
    pub(crate) sending_duration: Duration,

    /// How long to wait for leftover packets once sending has finished.
    pub(crate) waiting_duration: Duration,

    /// Number of packets dispatched in a single batch. Together with [`Self::target_rate`] this
    /// determines the inter-batch interval.
    pub(crate) sending_batch_size: usize,
}

impl ProbeProfile {
    /// The stress profile: send at `target_rate` for `sending_duration`, with the packet count
    /// derived from the two.
    pub(crate) fn stress(
        target_rate: NonZeroUsize,
        sending_duration: Duration,
        waiting_duration: Duration,
        sending_batch_size: NonZeroUsize,
    ) -> Self {
        ProbeProfile {
            expected_packets: (target_rate.get() as f32 * sending_duration.as_secs_f32()).floor()
                as usize,
            target_rate: target_rate.get(),
            sending_duration,
            waiting_duration,
            sending_batch_size: sending_batch_size.get(),
        }
    }

    /// The liveness profile: send exactly `expected_packets` to the target at `target_rate`, with
    /// the send window derived from the two.
    pub(crate) fn liveness(
        expected_packets: NonZeroUsize,
        target_rate: NonZeroUsize,
        waiting_duration: Duration,
    ) -> Self {
        let expected_packets = expected_packets.get();
        let target_rate = target_rate.get();

        ProbeProfile {
            expected_packets,
            target_rate,
            sending_duration: Duration::from_secs_f64(expected_packets as f64 / target_rate as f64),
            waiting_duration,
            // derived, rather than inherited from the stress profile or taken as a knob. the
            // inter-batch interval is batch/rate, so a batch that is large relative to the count
            // collapses the send window into a couple of bursts: the stress batch of 50 at 10
            // packets/second would send 100 packets as two bursts five seconds apart. liveness
            // measures delivery, so putting the target under load is the one thing it must not do.
            // deriving it from the count instead pins the PACING (ten dispatches across the window)
            // rather than a packet count that only holds for today's provisional defaults
            sending_batch_size: (expected_packets / LIVENESS_DISPATCHES_PER_PROBE).max(1),
        }
    }

    /// Time between consecutive batch dispatches needed to sustain [`Self::target_rate`]:
    /// `sending_batch_size / target_rate` seconds.
    pub(crate) fn batch_interval(&self) -> Duration {
        Duration::from_secs_f64(self.sending_batch_size as f64 / self.target_rate as f64)
    }
}

/// Configuration for the [`NodeStressTester`], controlling packet sending behaviour during a test run.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NodeTesterConfig {
    /// The sending knobs applied to a stress assignment.
    pub(crate) stress_profile: ProbeProfile,

    /// The sending knobs applied to each target of a liveness assignment.
    pub(crate) liveness_profile: ProbeProfile,

    /// Hard deadline for ONE target of a liveness wave. A wave probes its targets concurrently, so
    /// its duration is bounded by this rather than by the sum over the wave, and it must therefore
    /// sit inside the lease the orchestrator stamped on the whole assignment.
    pub(crate) liveness_per_target_timeout: Duration,

    /// How long the target node should delay the packet (i.e. the sphinx delay)
    pub(crate) packet_delay: Duration,

    /// Timeout for establishing the egress connection to the node under test.
    pub(crate) egress_connection_timeout: Duration,

    /// Timeout for the completing the noise handshake.
    pub(crate) noise_handshake_timeout: Duration,

    /// Whether the agent should reuse the same header for all packets, and consequently replay them.
    pub(crate) reuse_header: bool,

    /// Local socket address the agent binds its mixnet listener on to receive returning packets.
    pub(crate) mixnet_bind_address: SocketAddr,

    /// The ipv4 mixnet address announced in the contract, where the tested nodes will send their packets to.
    pub(crate) external_mixnet_address_v4: SocketAddr,

    /// The ipv6 mixnet address announced in the contract, where the tested nodes will send their packets to.
    pub(crate) external_mixnet_address_v6: SocketAddr,
}

impl NodeTesterConfig {
    /// The address the tested node should return the test packets to, encoded as the final hop of
    /// the sphinx route. It follows the family the node itself is being reached over, so that a
    /// test run exercises the same family in both directions rather than measuring the node's
    /// ipv6 ingress and its ipv4 egress.
    pub(crate) fn return_address_for(&self, tested_node_address: SocketAddr) -> SocketAddr {
        self.announced_addresses()
            .matching_family(tested_node_address)
    }

    /// The pair of addresses this agent has announced to the orchestrator.
    pub(crate) fn announced_addresses(&self) -> AgentMixAddresses {
        AgentMixAddresses {
            v4: self.external_mixnet_address_v4,
            v6: self.external_mixnet_address_v6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn nonzero(value: usize) -> NonZeroUsize {
        NonZeroUsize::new(value).expect("test used a zero where the type forbids it")
    }

    fn stress(target_rate: usize, sending_duration: Duration, batch_size: usize) -> ProbeProfile {
        ProbeProfile::stress(
            nonzero(target_rate),
            sending_duration,
            Duration::from_secs(5),
            nonzero(batch_size),
        )
    }

    fn liveness(expected_packets: usize, target_rate: usize) -> ProbeProfile {
        ProbeProfile::liveness(
            nonzero(expected_packets),
            nonzero(target_rate),
            Duration::from_secs(5),
        )
    }

    // the stress profile is specified as a rate held for a duration, with the count falling out
    #[test]
    fn stress_derives_its_packet_count_from_rate_and_duration() {
        let profile = stress(1000, Duration::from_secs(30), 50);
        assert_eq!(profile.expected_packets, 30_000);
        assert_eq!(profile.sending_duration, Duration::from_secs(30));
    }

    #[test]
    fn expected_packets_floors_fractional_result() {
        // 1000 * 0.5s = 500.0 — exact, no rounding needed
        assert_eq!(
            stress(1000, Duration::from_millis(500), 50).expected_packets,
            500
        );
    }

    #[test]
    fn expected_packets_floors_not_rounds() {
        // 3 * 1.5s = 4.5 -> 4, so a fractional product truncates rather than rounding up
        assert_eq!(
            stress(3, Duration::from_millis(1500), 50).expected_packets,
            4
        );
    }

    // the liveness profile inverts it: the count is the knob and the send window derives from it
    #[test]
    fn liveness_derives_its_send_window_from_the_packet_count() {
        let profile = liveness(100, 10);
        assert_eq!(profile.expected_packets, 100);
        assert_eq!(profile.sending_duration, Duration::from_secs(10));
    }

    #[test]
    fn batch_interval_is_batch_size_over_rate() {
        // 100 packets / 1000 pps = 100ms
        let interval = stress(1000, Duration::from_secs(30), 100).batch_interval();
        assert_eq!(interval, Duration::from_millis(100));
    }

    #[test]
    fn batch_interval_smaller_than_one_ms() {
        // 1 packet / 1000 pps = 1ms
        let interval = stress(1000, Duration::from_secs(30), 1).batch_interval();
        assert_eq!(interval, Duration::from_millis(1));
    }

    // a liveness probe must be PACED rather than bursted: it exists to measure delivery, so putting
    // the target under load is the one thing it must not do. inheriting the stress batch of 50
    // would send all 100 packets as two bursts five seconds apart, since the interval is
    // batch/rate. the batch is therefore derived rather than inherited or configured
    #[test]
    fn a_liveness_probe_is_paced_in_milliseconds_not_seconds() {
        // the profile's own values: 50 packets at 50/s, so five dispatches of ten over one second
        let profile = liveness(50, 50);
        assert_eq!(profile.sending_batch_size, 10);
        assert_eq!(profile.batch_interval(), Duration::from_millis(200));
    }

    // the batch tracks the COUNT rather than being pinned to today's provisional numbers: a fixed
    // batch is what made the stress value wrong here in the first place, and every one of these
    // values is required to be tunable without a code change
    #[test]
    fn the_liveness_batch_tracks_the_packet_count() {
        assert_eq!(liveness(500, 50).sending_batch_size, 100);
        assert_eq!(liveness(30, 10).sending_batch_size, 6);
    }

    // a count smaller than the dispatch target still has to send at least one packet per tick,
    // rather than flooring to a batch of zero and sending nothing at all
    #[test]
    fn a_tiny_liveness_count_still_sends_one_packet_per_tick() {
        let profile = liveness(3, 10);
        assert_eq!(profile.sending_batch_size, 1);
        assert_eq!(profile.batch_interval(), Duration::from_millis(100));
    }
}
