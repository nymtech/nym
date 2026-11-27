// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::env::vars::*;
use crate::agent::config::FiveHopMixnetRunConfig;
use anyhow::bail;
use nym_network_monitor_route_client::config::{
    RouteNodeSelection, RouteSessionConfig, RouteTrafficConfig,
};
use std::time::Duration;
use url::Url;

/// The settings of a five-hop mixnet run and its sessions.
#[derive(clap::Args, Debug)]
pub(crate) struct FiveHopMixnetArgs {
    /// The nym-api five-hop mixnet sessions read the active set from.
    #[arg(long, env = NYM_NETWORK_MONITOR_AGENT_NYM_API_URL_ARG)]
    nym_api_url: Url,

    /// The sending time a five-hop mixnet session draws its own around, between two thirds and four thirds of it.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "40s", env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_SENDING_DURATION_ARG)]
    five_hop_liveness_sending_duration: Duration,

    /// The longest a five-hop mixnet session waits for packets and acks still in flight once sending has finished.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "10s", env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_WAITING_DURATION_ARG)]
    five_hop_liveness_waiting_duration: Duration,

    /// Deadline for a whole five-hop mixnet run; it must sit inside the orchestrator's five-hop mixnet run timeout.
    #[arg(long, value_parser = humantime::parse_duration, default_value = "2m", env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_RUN_TIMEOUT_ARG)]
    five_hop_liveness_run_timeout: Duration,

    /// Average delay between packets of a client's main stream, a default client's if unset; for experiments only.
    #[arg(long, hide = true, value_parser = humantime::parse_duration, env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_MESSAGE_SENDING_AVERAGE_DELAY_ARG)]
    five_hop_liveness_message_sending_average_delay: Option<Duration>,

    /// Average delay between packets of a client's loop cover stream, a default client's if unset; for experiments only.
    #[arg(long, hide = true, value_parser = humantime::parse_duration, env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_LOOP_COVER_TRAFFIC_AVERAGE_DELAY_ARG)]
    five_hop_liveness_loop_cover_traffic_average_delay: Option<Duration>,

    /// Average delay of a test packet at each hop, a default client's if unset; for experiments only.
    #[arg(long, hide = true, value_parser = humantime::parse_duration, env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_AVERAGE_PACKET_DELAY_ARG)]
    five_hop_liveness_average_packet_delay: Option<Duration>,

    /// Average delay of a SURB-ack at each hop, a default client's if unset; for experiments only.
    #[arg(long, hide = true, value_parser = humantime::parse_duration, env = NYM_NETWORK_MONITOR_AGENT_FIVE_HOP_LIVENESS_AVERAGE_ACK_DELAY_ARG)]
    five_hop_liveness_average_ack_delay: Option<Duration>,
}

impl FiveHopMixnetArgs {
    /// Builds the config of a five-hop mixnet run through the selected mixnodes, with unset traffic knobs at a default client's values.
    pub(crate) fn build_config(
        &self,
        node_selection: RouteNodeSelection,
    ) -> anyhow::Result<FiveHopMixnetRunConfig> {
        let defaults = RouteTrafficConfig::default();
        let traffic = RouteTrafficConfig {
            message_sending_average_delay: self
                .five_hop_liveness_message_sending_average_delay
                .unwrap_or(defaults.message_sending_average_delay),
            loop_cover_traffic_average_delay: self
                .five_hop_liveness_loop_cover_traffic_average_delay
                .unwrap_or(defaults.loop_cover_traffic_average_delay),
            average_packet_delay: self
                .five_hop_liveness_average_packet_delay
                .unwrap_or(defaults.average_packet_delay),
            average_ack_delay: self
                .five_hop_liveness_average_ack_delay
                .unwrap_or(defaults.average_ack_delay),
        };

        if self.five_hop_liveness_sending_duration.is_zero() {
            bail!("attempted to set the five-hop liveness sending duration to 0s")
        }
        if self.five_hop_liveness_waiting_duration.is_zero() {
            bail!("attempted to set the five-hop liveness waiting duration to 0s")
        }
        if self.five_hop_liveness_run_timeout.is_zero() {
            bail!("attempted to set the five-hop liveness run timeout to 0s")
        }
        // a zero stream delay makes every poisson sample zero, so a session would flood its gateway
        if traffic.message_sending_average_delay.is_zero() {
            bail!("attempted to set the five-hop liveness message sending average delay to 0s")
        }
        if traffic.loop_cover_traffic_average_delay.is_zero() {
            bail!("attempted to set the five-hop liveness loop cover traffic average delay to 0s")
        }

        Ok(FiveHopMixnetRunConfig {
            session: RouteSessionConfig {
                nym_api_url: self.nym_api_url.clone(),
                sending_duration: self.five_hop_liveness_sending_duration,
                waiting_duration: self.five_hop_liveness_waiting_duration,
                traffic,
                node_selection,
            },
            timeout: self.five_hop_liveness_run_timeout,
        })
    }
}
