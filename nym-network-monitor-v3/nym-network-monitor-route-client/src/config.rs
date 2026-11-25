// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_client_core_config_types::{Acknowledgements, CoverTraffic, Traffic};
use rand::RngExt;
use std::time::Duration;
use url::Url;

/// What every route session runs with.
#[derive(Debug, Clone)]
pub struct RouteSessionConfig {
    /// The nym-api a session reads the active set from.
    pub nym_api_url: Url,

    /// The configured sending time, around which each session draws its own.
    pub sending_duration: Duration,

    /// How long a session waits for packets and acks still in flight once sending has finished.
    pub waiting_duration: Duration,

    /// The traffic a session sends.
    pub traffic: RouteTrafficConfig,

    /// Which mixnodes a session routes through; only the manual command changes it.
    pub node_selection: RouteNodeSelection,
}

impl RouteSessionConfig {
    /// A session's sending time, drawn uniformly between two thirds and four thirds of the configured one.
    pub fn draw_sending_time(&self) -> Duration {
        rand::rng().random_range(self.sending_duration * 2 / 3..=self.sending_duration * 4 / 3)
    }
}

/// Which mixnodes a session routes through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RouteNodeSelection {
    /// The mixnodes of the active set, in their layers, as a client routes.
    #[default]
    ActiveSet,

    /// The active set and the standby nodes of the rewarded set, for experiments.
    RewardedSet,

    /// Every node declaring the mixnode role, for experiments.
    AllMixnodes,
}

/// The rates and delays of a session's traffic, a default client's unless changed for an experiment.
#[derive(Debug, Clone, Copy)]
pub struct RouteTrafficConfig {
    /// Average delay between packets of a client's main stream.
    pub message_sending_average_delay: Duration,

    /// Average delay between packets of a client's loop cover stream.
    pub loop_cover_traffic_average_delay: Duration,

    /// Average delay of a test packet at each hop.
    pub average_packet_delay: Duration,

    /// Average delay of a SURB-ack at each hop.
    pub average_ack_delay: Duration,
}

impl Default for RouteTrafficConfig {
    fn default() -> Self {
        let traffic = Traffic::default();
        let cover = CoverTraffic::default();
        let acks = Acknowledgements::default();
        RouteTrafficConfig {
            message_sending_average_delay: traffic.message_sending_average_delay,
            loop_cover_traffic_average_delay: cover.loop_cover_traffic_average_delay,
            average_packet_delay: traffic.average_packet_delay,
            average_ack_delay: acks.average_ack_delay,
        }
    }
}
