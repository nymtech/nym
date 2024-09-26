// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::MixnodePaths;
use crate::config::Config;
use crate::error::MixnodeError;
use clap::crate_version;
use nym_config::defaults::DEFAULT_VERLOC_LISTENING_PORT;
use nym_config::helpers::inaddr_any;
use nym_config::serde_helpers::de_maybe_port;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

pub const DEFAULT_VERLOC_PORT: u16 = DEFAULT_VERLOC_LISTENING_PORT;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixnodeConfig {
    pub storage_paths: MixnodePaths,

    pub verloc: Verloc,

    #[serde(default)]
    pub debug: Debug,
}

impl MixnodeConfig {
    pub fn new_default() -> Self {
        MixnodeConfig {
            storage_paths: MixnodePaths {},
            verloc: Default::default(),
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Verloc {
    /// Socket address this node will use for binding its verloc API.
    /// default: `0.0.0.0:1790`
    pub bind_address: SocketAddr,

    /// If applicable, custom port announced in the self-described API that other clients and nodes
    /// will use.
    /// Useful when the node is behind a proxy.
    #[serde(deserialize_with = "de_maybe_port")]
    pub announce_port: Option<u16>,

    #[serde(default)]
    pub debug: VerlocDebug,
}

impl Default for Verloc {
    fn default() -> Self {
        Verloc {
            bind_address: SocketAddr::new(inaddr_any(), DEFAULT_VERLOC_PORT),
            announce_port: None,
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerlocDebug {
    /// Specifies number of echo packets sent to each node during a measurement run.
    pub packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the connection to get established.
    #[serde(with = "humantime_serde")]
    pub connection_timeout: Duration,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    #[serde(with = "humantime_serde")]
    pub packet_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    #[serde(with = "humantime_serde")]
    pub delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    pub tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    #[serde(with = "humantime_serde")]
    pub testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    #[serde(with = "humantime_serde")]
    pub retry_timeout: Duration,
}

impl VerlocDebug {
    const DEFAULT_PACKETS_PER_NODE: usize = 100;
    const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
    const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
    const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
    const DEFAULT_BATCH_SIZE: usize = 50;
    const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
    const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);
}

impl Default for VerlocDebug {
    fn default() -> Self {
        VerlocDebug {
            packets_per_node: VerlocDebug::DEFAULT_PACKETS_PER_NODE,
            connection_timeout: VerlocDebug::DEFAULT_CONNECTION_TIMEOUT,
            packet_timeout: VerlocDebug::DEFAULT_PACKET_TIMEOUT,
            delay_between_packets: VerlocDebug::DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: VerlocDebug::DEFAULT_BATCH_SIZE,
            testing_interval: VerlocDebug::DEFAULT_TESTING_INTERVAL,
            retry_timeout: VerlocDebug::DEFAULT_RETRY_TIMEOUT,
        }
    }
}

impl From<VerlocDebug> for nym_mixnode::config::Verloc {
    fn from(value: VerlocDebug) -> Self {
        nym_mixnode::config::Verloc {
            packets_per_node: value.packets_per_node,
            connection_timeout: value.connection_timeout,
            packet_timeout: value.packet_timeout,
            delay_between_packets: value.delay_between_packets,
            tested_nodes_batch_size: value.tested_nodes_batch_size,
            testing_interval: value.testing_interval,
            retry_timeout: value.retry_timeout,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debug {
    /// Delay between each subsequent node statistics being logged to the console
    #[serde(with = "humantime_serde")]
    pub node_stats_logging_delay: Duration,

    /// Delay between each subsequent node statistics being updated
    #[serde(with = "humantime_serde")]
    pub node_stats_updating_delay: Duration,
}

impl Debug {
    const DEFAULT_NODE_STATS_LOGGING_DELAY: Duration = Duration::from_millis(60_000);
    const DEFAULT_NODE_STATS_UPDATING_DELAY: Duration = Duration::from_millis(30_000);
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            node_stats_logging_delay: Debug::DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: Debug::DEFAULT_NODE_STATS_UPDATING_DELAY,
        }
    }
}

// a temporary solution until all nodes are even more tightly integrated
pub fn ephemeral_mixnode_config(
    config: Config,
) -> Result<nym_mixnode::config::Config, MixnodeError> {
    let host = nym_mixnode::config::Host {
        public_ips: config.host.public_ips,
        hostname: config.host.hostname,
    };

    let http = nym_mixnode::config::Http {
        bind_address: config.http.bind_address,
        landing_page_assets_path: config.http.landing_page_assets_path,
        metrics_key: config.http.access_token,
    };

    let verloc_bind_ip = config.mixnode.verloc.bind_address.ip();
    let mix_bind_ip = config.mixnet.bind_address.ip();
    if verloc_bind_ip != mix_bind_ip {
        return Err(MixnodeError::UnsupportedAddresses {
            verloc_bind_ip,
            mix_bind_ip,
        });
    }

    let listening_address = mix_bind_ip;
    let mix_port = config.mixnet.bind_address.port();
    let verloc_port = config.mixnode.verloc.bind_address.port();
    let nym_api_urls = config.mixnet.nym_api_urls;

    let mixnode = nym_mixnode::config::MixNode {
        // that field is very much irrelevant, but I guess let's keep them for now
        version: format!("{}-nym-node", crate_version!()),
        id: config.id,
        listening_address,
        mix_port,
        verloc_port,
        nym_api_urls,
    };

    Ok(nym_mixnode::config::Config::externally_loaded(
        host,
        http,
        mixnode,
        nym_mixnode::config::MixNodePaths::new_empty(),
        config.mixnode.verloc.debug,
        config.logging,
        nym_mixnode::config::Debug {
            node_stats_logging_delay: config.mixnode.debug.node_stats_logging_delay,
            node_stats_updating_delay: config.mixnode.debug.node_stats_updating_delay,
            packet_forwarding_initial_backoff: config
                .mixnet
                .debug
                .packet_forwarding_initial_backoff,
            packet_forwarding_maximum_backoff: config
                .mixnet
                .debug
                .packet_forwarding_maximum_backoff,
            initial_connection_timeout: config.mixnet.debug.initial_connection_timeout,
            maximum_connection_buffer_size: config.mixnet.debug.maximum_connection_buffer_size,
            use_legacy_framed_packet_version: false,
        },
    ))
}
