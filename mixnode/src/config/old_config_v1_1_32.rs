// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config;
use crate::config::persistence::paths::MixNodePaths;
use crate::config::{Config, Debug, MixNode, Verloc};
use nym_bin_common::logging::LoggingSettings;
use nym_config::{
    must_get_home, read_config_from_toml_file, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

const DEFAULT_MIXNODES_DIR: &str = "mixnodes";

// 'RTT MEASUREMENT'
const DEFAULT_PACKETS_PER_NODE: usize = 100;
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
const DEFAULT_BATCH_SIZE: usize = 50;
const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);

// 'DEBUG'
const DEFAULT_NODE_STATS_LOGGING_DELAY: Duration = Duration::from_millis(60_000);
const DEFAULT_NODE_STATS_UPDATING_DELAY: Duration = Duration::from_millis(30_000);
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 2000;

/// Derive default path to mixnodes's config directory.
/// It should get resolved to `$HOME/.nym/mixnodes/<id>/config`
fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_MIXNODES_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to mixnodes's config file.
/// It should get resolved to `$HOME/.nym/mixnodes/<id>/config/config.toml`
fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_32 {
    pub mixnode: MixNodeV1_1_32,

    // i hope this laziness is not going to backfire...
    pub storage_paths: MixNodePaths,

    #[serde(default)]
    pub verloc: VerlocV1_1_32,

    #[serde(default)]
    pub logging: LoggingSettings,

    #[serde(default)]
    pub debug: DebugV1_1_32,
}

impl ConfigV1_1_32 {
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        read_config_from_toml_file(default_config_filepath(id))
    }
}

impl From<ConfigV1_1_32> for Config {
    fn from(value: ConfigV1_1_32) -> Self {
        Config {
            // \/ ADDED
            save_path: None,
            // /\ ADDED

            // \/ ADDED
            host: config::Host {
                // this is a very bad default!
                public_ips: vec![value.mixnode.listening_address],
                hostname: None,
            },
            // /\ ADDED

            // \/ ADDED
            http: config::Http {
                bind_address: SocketAddr::new(
                    value.mixnode.listening_address,
                    value.mixnode.http_api_port,
                ),
                landing_page_assets_path: None,
                metrics_key: None,
            },
            // /\ ADDED
            mixnode: MixNode {
                version: value.mixnode.version,
                id: value.mixnode.id,
                listening_address: value.mixnode.listening_address,
                mix_port: value.mixnode.mix_port,
                verloc_port: value.mixnode.verloc_port,
                nym_api_urls: value.mixnode.nym_api_urls,
            },
            storage_paths: value.storage_paths,
            verloc: value.verloc.into(),
            logging: value.logging,
            debug: value.debug.into(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct MixNodeV1_1_32 {
    /// Version of the mixnode for which this configuration was created.
    pub version: String,

    /// ID specifies the human readable ID of this particular mixnode.
    pub id: String,

    /// Address to which this mixnode will bind to and will be listening for packets.
    pub listening_address: IpAddr,

    /// Port used for listening for all mixnet traffic.
    /// (default: 1789)
    pub mix_port: u16,

    /// Port used for listening for verloc traffic.
    /// (default: 1790)
    pub verloc_port: u16,

    /// Port used for listening for http requests.
    /// (default: 8000)
    pub http_api_port: u16,

    /// Addresses to nym APIs from which the node gets the view of the network.
    pub nym_api_urls: Vec<Url>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerlocV1_1_32 {
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

impl From<VerlocV1_1_32> for Verloc {
    fn from(value: VerlocV1_1_32) -> Self {
        Verloc {
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

impl Default for VerlocV1_1_32 {
    fn default() -> Self {
        VerlocV1_1_32 {
            packets_per_node: DEFAULT_PACKETS_PER_NODE,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            packet_timeout: DEFAULT_PACKET_TIMEOUT,
            delay_between_packets: DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: DEFAULT_BATCH_SIZE,
            testing_interval: DEFAULT_TESTING_INTERVAL,
            retry_timeout: DEFAULT_RETRY_TIMEOUT,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DebugV1_1_32 {
    /// Delay between each subsequent node statistics being logged to the console
    #[serde(with = "humantime_serde")]
    pub node_stats_logging_delay: Duration,

    /// Delay between each subsequent node statistics being updated
    #[serde(with = "humantime_serde")]
    pub node_stats_updating_delay: Duration,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    pub packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    #[serde(with = "humantime_serde")]
    pub initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    pub maximum_connection_buffer_size: usize,

    /// Specifies whether the mixnode should be using the legacy framing for the sphinx packets.
    // it's set to true by default. The reason for that decision is to preserve compatibility with the
    // existing nodes whilst everyone else is upgrading and getting the code for handling the new field.
    // It shall be disabled in the subsequent releases.
    pub use_legacy_framed_packet_version: bool,
}

impl From<DebugV1_1_32> for Debug {
    fn from(value: DebugV1_1_32) -> Self {
        Debug {
            node_stats_logging_delay: value.node_stats_logging_delay,
            node_stats_updating_delay: value.node_stats_updating_delay,
            packet_forwarding_initial_backoff: value.packet_forwarding_initial_backoff,
            packet_forwarding_maximum_backoff: value.packet_forwarding_maximum_backoff,
            initial_connection_timeout: value.initial_connection_timeout,
            maximum_connection_buffer_size: value.maximum_connection_buffer_size,
            use_legacy_framed_packet_version: value.use_legacy_framed_packet_version,
        }
    }
}

impl Default for DebugV1_1_32 {
    fn default() -> Self {
        DebugV1_1_32 {
            node_stats_logging_delay: DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: DEFAULT_NODE_STATS_UPDATING_DELAY,
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            use_legacy_framed_packet_version: false,
        }
    }
}
