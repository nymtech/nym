// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use nym_client_core::client::topology_control::nym_api_provider::NymApiTopologyProvider;
use nym_client_core_config_types::{Acknowledgements, CoverTraffic, Topology, Traffic};
use nym_sphinx::params::PacketSize;
use nym_topology::{NymRouteProvider, TopologyProvider};
use rand::RngExt;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// What every route session runs with.
#[derive(Debug, Clone)]
pub struct RouteSessionConfig {
    /// The nym-api a session reads the active set from.
    pub nym_api_url: Url,

    /// The configured sending time, around which each session draws its own.
    pub sending_duration: Duration,

    /// The longest a session waits for packets and acks still in flight once sending has finished.
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

    /// Reads the network once: the topology provider's default read for the active set, its extended read otherwise.
    pub(crate) async fn read_topology(&self) -> anyhow::Result<NymRouteProvider> {
        let topology = self
            .topology_provider()?
            .get_new_topology()
            .await
            .context("could not read the network from nym-api")?;

        Ok(NymRouteProvider::new(
            Arc::new(topology),
            self.topology_config().ignore_egress_epoch_role,
        ))
    }

    /// The client stack's nym-api topology provider, with the client's default settings.
    pub(crate) fn topology_provider(&self) -> anyhow::Result<NymApiTopologyProvider> {
        Ok(NymApiTopologyProvider::new(
            self.topology_config(),
            self.nym_api_client()?,
        ))
    }

    /// The HTTP client the topology provider reads nym-api through, built as the monitor's other clients are.
    pub(crate) fn nym_api_client(&self) -> anyhow::Result<nym_http_api_client::Client> {
        Ok(
            nym_http_api_client::Client::builder(self.nym_api_url.clone())?
                .no_hickory_dns()
                .with_user_agent(format!(
                    "nym-network-monitor-route-client/{}",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()?,
        )
    }

    /// The client's default topology settings, with the extended read for a selection wider than the active set.
    fn topology_config(&self) -> Topology {
        Topology {
            use_extended_topology: self.node_selection != RouteNodeSelection::ActiveSet,
            ..Topology::default()
        }
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

impl RouteTrafficConfig {
    /// Average delay between packets of the main and cover streams sent as one Poisson stream.
    pub fn combined_average_delay(&self) -> Duration {
        let main_rate = 1.0 / self.message_sending_average_delay.as_secs_f64();
        let cover_rate = 1.0 / self.loop_cover_traffic_average_delay.as_secs_f64();
        Duration::from_secs_f64(1.0 / (main_rate + cover_rate))
    }

    /// The client's default packet size, which is not a knob.
    pub fn packet_size(&self) -> PacketSize {
        Traffic::default().primary_packet_size
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_streams_combine_at_the_sum_of_their_rates() {
        let traffic = RouteTrafficConfig {
            message_sending_average_delay: Duration::from_millis(20),
            loop_cover_traffic_average_delay: Duration::from_millis(200),
            ..RouteTrafficConfig::default()
        };

        // 50 and 5 packets a second make 55
        assert_eq!(
            traffic.combined_average_delay(),
            Duration::from_secs(1) / 55
        );
    }
}
