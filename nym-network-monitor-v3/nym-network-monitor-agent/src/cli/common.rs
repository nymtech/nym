// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::config::{NodeTesterConfig, ProbeProfile};
use anyhow::bail;
use nym_network_monitor_orchestrator_requests::models::AgentMixAddresses;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::time::Duration;

#[derive(clap::Args, Debug)]
pub(crate) struct CommonArgs {
    /// Specifies for how long the agent should be sending test packets with the specified rate.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "30s", env = NYM_NETWORK_MONITOR_AGENT_SENDING_DURATION_ARG)]
    sending_duration: Duration,

    /// Specifies how long the agent will wait to receive any leftover packets after finishing sending.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "5s", env = NYM_NETWORK_MONITOR_AGENT_WAITING_DURATION_ARG)]
    waiting_duration: Duration,

    /// How long the node itself should delay the packet
    /// It shouldn't be set to zero as otherwise the node will not put the packet through
    /// its delay queue and we would not test the entire pipeline
    #[arg(long, value_parser = humantime::parse_duration, default_value = "50ms", env = NYM_NETWORK_MONITOR_AGENT_PACKET_DELAY_ARG)]
    packet_delay: Duration,

    /// Specifies the target rate of packets (per second) to be sent.
    #[arg(long, default_value = "1000", env = NYM_NETWORK_MONITOR_AGENT_TARGET_RATE_ARG)]
    target_rate: NonZeroUsize,

    /// Specifies whether the agent should reuse the same header for all packets.
    /// And consequently replay them
    #[arg(long, short, default_value = "true", env = NYM_NETWORK_MONITOR_AGENT_REUSE_HEADER_ARG)]
    reuse_header: bool,

    /// Timeout for establishing the TCP connection to the node under test.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "5s", env = NYM_NETWORK_MONITOR_AGENT_EGRESS_CONNECTION_TIMEOUT_ARG)]
    egress_connection_timeout: Duration,

    /// Timeout for completing the Noise handshake with the node under test.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "3s", env = NYM_NETWORK_MONITOR_AGENT_NOISE_HANDSHAKE_TIMEOUT_ARG)]
    noise_handshake_timeout: Duration,

    /// Number of packets sent in a single batch. Together with `target_rate` this controls
    /// how frequently batches are dispatched: one batch every `sending_batch_size / target_rate` seconds.
    #[arg(long, default_value = "50", env = NYM_NETWORK_MONITOR_AGENT_SENDING_BATCH_SIZE_ARG)]
    sending_batch_size: NonZeroUsize,

    /// Specifies the path to the noise key file used for establishing tunnel with the node being tested
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NOISE_KEY_PATH_ARG)]
    pub(crate) noise_key_path: String,

    /// Specifies the socket address the agent will bind to for receiving mixnet traffic.
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_BIND_ADDRESS_ARG, default_value = "[::]:9000")]
    bind_address: SocketAddr,

    /// Number of test packets a liveness probe sends to EACH target of a wave. This is the primary
    /// knob of the liveness profile: its send window derives from this and the liveness target rate
    /// rather than being configured, inverting the stress profile's rate-times-duration shape.
    #[arg(long, default_value = "50", env = NYM_NETWORK_MONITOR_AGENT_LIVENESS_PACKETS_ARG)]
    liveness_packets: NonZeroUsize,

    /// Target rate of packets (per second) a liveness probe sends to EACH target of a wave,
    /// independent of how many targets the wave carries. An order of magnitude below the stress
    /// rate, but high enough that a target is done sending within about a second: liveness asks
    /// whether a node forwards at all, so there is no reason to hold a target open longer. The
    /// aggregate the agent emits is therefore this times the wave width, and scales with the
    /// orchestrator's wave size rather than being capped here.
    #[arg(long, default_value = "50", env = NYM_NETWORK_MONITOR_AGENT_LIVENESS_TARGET_RATE_ARG)]
    liveness_target_rate: NonZeroUsize,

    /// How long a liveness probe waits for leftover packets from a target after it has finished
    /// sending to it.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "5s", env = NYM_NETWORK_MONITOR_AGENT_LIVENESS_WAITING_DURATION_ARG)]
    liveness_waiting_duration: Duration,

    /// Hard deadline for ONE target of a liveness wave. It must sit inside the orchestrator's
    /// liveness lease, because a wave probes its targets concurrently and so is bounded by this
    /// rather than by the sum over the wave.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "30s", env = NYM_NETWORK_MONITOR_AGENT_LIVENESS_PER_TARGET_TIMEOUT_ARG)]
    liveness_per_target_timeout: Duration,
}

impl CommonArgs {
    /// Constructs a [`NodeTesterConfig`] from the common CLI arguments.
    /// `mixnet_address` is provided separately as it is command-specific.
    pub(crate) fn build_config(
        &self,
        external_address_v4: SocketAddr,
        external_address_v6: SocketAddr,
    ) -> anyhow::Result<NodeTesterConfig> {
        // fail here rather than at the first announcement: the orchestrator rejects anything but a
        // plain ipv4/ipv6 pair, since a swapped, duplicated or ipv4-mapped address would authorise
        // an ingress the tested nodes will never actually see us coming from
        let announced = AgentMixAddresses {
            v4: external_address_v4,
            v6: external_address_v6,
        };
        if !announced.has_distinct_families() {
            bail!(
                "the announced mixnet addresses must be a plain ipv4/ipv6 pair, got {external_address_v4} and {external_address_v6}"
            )
        }

        if self.sending_duration.is_zero() {
            bail!("attempted to set sending duration to 0s")
        }
        if self.egress_connection_timeout.is_zero() {
            bail!("attempted to set egress connection timeout to 0s")
        }
        if self.noise_handshake_timeout.is_zero() {
            bail!("attempted to set noise handshake timeout to 0s")
        }
        if self.liveness_per_target_timeout.is_zero() {
            bail!("attempted to set the liveness per-target timeout to 0s")
        }

        Ok(NodeTesterConfig {
            stress_profile: ProbeProfile::stress(
                self.target_rate,
                self.sending_duration,
                self.waiting_duration,
                self.sending_batch_size,
            ),
            liveness_profile: ProbeProfile::liveness(
                self.liveness_packets,
                self.liveness_target_rate,
                self.liveness_waiting_duration,
            ),
            liveness_per_target_timeout: self.liveness_per_target_timeout,
            packet_delay: self.packet_delay,
            egress_connection_timeout: self.egress_connection_timeout,
            noise_handshake_timeout: self.noise_handshake_timeout,
            reuse_header: self.reuse_header,
            mixnet_bind_address: self.bind_address,
            external_mixnet_address_v4: external_address_v4,
            external_mixnet_address_v6: external_address_v6,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // `CommonArgs` is an argument group, so it needs a parser root to be exercised on its own
    #[derive(Parser)]
    struct TestCli {
        #[clap(flatten)]
        args: CommonArgs,
    }

    /// The one argument with no default, which every parse has to supply.
    const REQUIRED: &[&str] = &["test-agent", "--noise-key-path", "/var/lib/nym/noise.pem"];

    fn try_build(overrides: &[&str]) -> anyhow::Result<NodeTesterConfig> {
        let argv: Vec<&str> = REQUIRED.iter().chain(overrides.iter()).copied().collect();
        TestCli::try_parse_from(argv)
            .expect("failed to parse arguments")
            .args
            .build_config(
                "1.2.3.4:9000".parse().expect("bad test v4 address"),
                "[2001:db8::1]:9000".parse().expect("bad test v6 address"),
            )
    }

    fn parse(overrides: &[&str]) -> NodeTesterConfig {
        try_build(overrides).expect("failed to build the config")
    }

    // every liveness value is provisional, so pinning the documented defaults is what catches one
    // of them drifting away from the design without the design being amended
    #[test]
    fn the_liveness_profile_carries_its_documented_defaults() {
        let config = parse(&[]);
        let liveness = config.liveness_profile;

        assert_eq!(liveness.expected_packets, 50);
        assert_eq!(liveness.target_rate, 50);
        assert_eq!(liveness.waiting_duration, Duration::from_secs(5));
        // all three derived rather than configured: 50 packets at 50 per second is a one second
        // send window, dispatched as five batches of ten 200ms apart, so the probe is paced at
        // millisecond granularity rather than being spread over seconds or sent as a burst
        assert_eq!(liveness.sending_duration, Duration::from_secs(1));
        assert_eq!(liveness.sending_batch_size, 10);
        assert_eq!(liveness.batch_interval(), Duration::from_millis(200));
        // has to sit inside the orchestrator's one minute lease, with room for the send window,
        // the straggler wait and connection setup
        assert_eq!(config.liveness_per_target_timeout, Duration::from_secs(30));
    }

    // splitting the two profiles must not move the stress test, whose values are already deployed
    #[test]
    fn the_stress_profile_keeps_its_existing_defaults() {
        let stress = parse(&[]).stress_profile;

        assert_eq!(stress.target_rate, 1000);
        assert_eq!(stress.sending_duration, Duration::from_secs(30));
        assert_eq!(stress.waiting_duration, Duration::from_secs(5));
        assert_eq!(stress.sending_batch_size, 50);
        assert_eq!(stress.expected_packets, 30_000);
    }

    // an agent host that cannot sustain a full wave must be a configuration change, so every one of
    // these has to be movable without a rebuild
    #[test]
    fn every_liveness_knob_is_overridable() {
        let config = parse(&[
            "--liveness-packets",
            "7",
            "--liveness-target-rate",
            "3",
            "--liveness-waiting-duration",
            "2s",
            "--liveness-per-target-timeout",
            "9s",
        ]);

        assert_eq!(config.liveness_profile.expected_packets, 7);
        assert_eq!(config.liveness_profile.target_rate, 3);
        assert_eq!(
            config.liveness_profile.waiting_duration,
            Duration::from_secs(2)
        );
        assert_eq!(config.liveness_per_target_timeout, Duration::from_secs(9));
    }

    // a zero deadline would time out every target of every wave the instant it began, scoring the
    // whole population zero. same class as the existing zero-duration guards, and it has to be
    // caught when the config is built rather than at parse time, since "0s" parses fine
    #[test]
    fn a_zero_liveness_per_target_timeout_is_rejected() {
        assert!(try_build(&["--liveness-per-target-timeout", "0s"]).is_err());
    }

    // a probe that sends no packets, or sends them at no rate, measures nothing and would divide by
    // zero deriving its send window. asserted per flag, since one keeping its NonZero parser proves
    // nothing about the other
    #[test]
    fn a_zero_liveness_packet_count_or_rate_is_rejected() {
        for flag in ["--liveness-packets", "--liveness-target-rate"] {
            let argv: Vec<&str> = REQUIRED.iter().copied().chain([flag, "0"]).collect();
            assert!(
                TestCli::try_parse_from(argv).is_err(),
                "{flag} accepted a zero"
            );
        }
    }
}
