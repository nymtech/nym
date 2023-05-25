// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::pathfinder::MixNodePathfinder;
use crate::config::template::CONFIG_TEMPLATE;
use nym_bin_common::logging::LoggingSettings;
use nym_config::defaults::{
    mainnet, DEFAULT_HTTP_API_LISTENING_PORT, DEFAULT_MIX_LISTENING_PORT,
    DEFAULT_VERLOC_LISTENING_PORT,
};
use nym_config::helpers::inaddr_any;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use url::Url;

pub mod persistence;
mod template;

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

/// Derive default path to mixnodes's config file.
/// It should get resolved to `$HOME/.nym/mixnodes/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_MIXNODES_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
        .join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to mixnodes's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/mixnodes/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_MIXNODES_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mixnode: MixNode,

    pub paths: MixNodePathfinder,

    #[serde(default)]
    pub verloc: Verloc,

    #[serde(default)]
    pub logging: LoggingSettings,

    #[serde(default)]
    pub debug: Debug,
}

impl NymConfigTemplate for Config {
    fn template() -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S) -> Self {
        Config {
            mixnode: MixNode::new_default(id.as_ref()),
            paths: MixNodePathfinder::new_default(id.as_ref()),
            verloc: Default::default(),
            logging: Default::default(),
            debug: Default::default(),
        }
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.mixnode.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    // builder methods
    pub fn with_custom_nym_apis(mut self, nym_api_urls: Vec<Url>) -> Self {
        self.mixnode.nym_api_urls = nym_api_urls;
        self
    }

    pub fn with_listening_address(mut self, listening_address: IpAddr) -> Self {
        self.mixnode.listening_address = listening_address;
        self
    }

    pub fn with_mix_port(mut self, port: u16) -> Self {
        self.mixnode.mix_port = port;
        self
    }

    pub fn with_verloc_port(mut self, port: u16) -> Self {
        self.mixnode.verloc_port = port;
        self
    }

    pub fn with_http_api_port(mut self, port: u16) -> Self {
        self.mixnode.http_api_port = port;
        self
    }

    pub fn with_custom_version<S: Into<String>>(mut self, version: S) -> Self {
        self.mixnode.version = version.into();
        self
    }

    pub fn get_nym_api_endpoints(&self) -> Vec<Url> {
        self.mixnode.nym_api_urls.clone()
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct MixNode {
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

impl MixNode {
    pub fn new_default<S: Into<String>>(id: S) -> Self {
        MixNode {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: id.into(),
            listening_address: inaddr_any(),
            mix_port: DEFAULT_MIX_LISTENING_PORT,
            verloc_port: DEFAULT_VERLOC_LISTENING_PORT,
            http_api_port: DEFAULT_HTTP_API_LISTENING_PORT,
            nym_api_urls: vec![Url::from_str(mainnet::NYM_API).expect("Invalid default API URL")],
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Verloc {
    /// Specifies number of echo packets sent to each node during a measurement run.
    pub packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the connection to get established.
    pub connection_timeout: Duration,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    pub packet_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    pub delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    pub tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    pub testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    pub retry_timeout: Duration,
}

impl Default for Verloc {
    fn default() -> Self {
        Verloc {
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
pub struct Debug {
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

impl Default for Debug {
    fn default() -> Self {
        Debug {
            node_stats_logging_delay: DEFAULT_NODE_STATS_LOGGING_DELAY,
            node_stats_updating_delay: DEFAULT_NODE_STATS_UPDATING_DELAY,
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            // TODO: remember to change it in one of future releases!!
            use_legacy_framed_packet_version: true,
        }
    }
}
