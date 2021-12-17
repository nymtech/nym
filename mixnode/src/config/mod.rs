// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::config_template;
use config::defaults::*;
use config::NymConfig;
use serde::{Deserialize, Deserializer, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

pub mod persistence;
mod template;

pub(crate) const MISSING_VALUE: &str = "MISSING VALUE";

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
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 128;

pub fn missing_string_value<T: From<String>>() -> T {
    MISSING_VALUE.to_string().into()
}

fn bind_all_address() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}

fn default_mix_port() -> u16 {
    DEFAULT_MIX_LISTENING_PORT
}

fn default_verloc_port() -> u16 {
    DEFAULT_VERLOC_LISTENING_PORT
}

fn default_http_api_port() -> u16 {
    DEFAULT_HTTP_API_LISTENING_PORT
}

// basically a migration helper that deserialises string representation of a maybe socket addr (like "1.1.1.1:1234")
// into just the ipaddr (like "1.1.1.1")
pub(super) fn de_ipaddr_from_maybe_str_socks_addr<'de, D>(
    deserializer: D,
) -> Result<IpAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if let Ok(socket_addr) = SocketAddr::from_str(&s) {
        Ok(socket_addr.ip())
    } else {
        IpAddr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    mixnode: MixNode,

    #[serde(default)]
    verloc: Verloc,
    #[serde(default)]
    logging: Logging,
    #[serde(default)]
    debug: Debug,
}

impl NymConfig for Config {
    fn template() -> &'static str {
        config_template()
    }

    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("mixnodes")
    }

    fn root_directory(&self) -> PathBuf {
        self.mixnode.nym_root_directory.clone()
    }

    fn config_directory(&self) -> PathBuf {
        self.mixnode
            .nym_root_directory
            .join(&self.mixnode.id)
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.mixnode
            .nym_root_directory
            .join(&self.mixnode.id)
            .join("data")
    }
}

impl Config {
    pub fn new<S: Into<String>>(id: S) -> Self {
        Config::default().with_id(id)
    }

    // builder methods
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        let id = id.into();
        if self
            .mixnode
            .private_identity_key_file
            .as_os_str()
            .is_empty()
        {
            self.mixnode.private_identity_key_file =
                self::MixNode::default_private_identity_key_file(&id);
        }
        if self.mixnode.public_identity_key_file.as_os_str().is_empty() {
            self.mixnode.public_identity_key_file =
                self::MixNode::default_public_identity_key_file(&id);
        }

        if self.mixnode.private_sphinx_key_file.as_os_str().is_empty() {
            self.mixnode.private_sphinx_key_file =
                self::MixNode::default_private_sphinx_key_file(&id);
        }
        if self.mixnode.public_sphinx_key_file.as_os_str().is_empty() {
            self.mixnode.public_sphinx_key_file =
                self::MixNode::default_public_sphinx_key_file(&id);
        }

        self.mixnode.id = id;
        self
    }

    pub fn with_custom_validator_apis(mut self, validator_api_urls: Vec<Url>) -> Self {
        self.mixnode.validator_api_urls = validator_api_urls;
        self
    }

    pub fn with_listening_address<S: Into<String>>(mut self, listening_address: S) -> Self {
        let listening_address_string = listening_address.into();
        if let Ok(ip_addr) = listening_address_string.parse() {
            self.mixnode.listening_address = ip_addr
        } else {
            error!(
                "failed to change listening address. the provided value ({}) was invalid",
                listening_address_string
            )
        }
        self
    }

    pub fn with_announce_address<S: Into<String>>(mut self, announce_address: S) -> Self {
        self.mixnode.announce_address = announce_address.into();
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

    pub fn announce_address_from_listening_address(mut self) -> Self {
        self.mixnode.announce_address = self.mixnode.listening_address.to_string();
        self
    }

    pub fn with_custom_version(mut self, version: &str) -> Self {
        self.mixnode.version = version.to_string();
        self
    }

    // getters
    pub fn get_id(&self) -> String {
        self.mixnode.id.clone()
    }

    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    pub fn get_private_identity_key_file(&self) -> PathBuf {
        self.mixnode.private_identity_key_file.clone()
    }

    pub fn get_public_identity_key_file(&self) -> PathBuf {
        self.mixnode.public_identity_key_file.clone()
    }

    pub fn get_private_sphinx_key_file(&self) -> PathBuf {
        self.mixnode.private_sphinx_key_file.clone()
    }

    pub fn get_public_sphinx_key_file(&self) -> PathBuf {
        self.mixnode.public_sphinx_key_file.clone()
    }

    pub fn get_validator_api_endpoints(&self) -> Vec<Url> {
        self.mixnode.validator_api_urls.clone()
    }

    pub fn get_node_stats_logging_delay(&self) -> Duration {
        self.debug.node_stats_logging_delay
    }

    pub fn get_node_stats_updating_delay(&self) -> Duration {
        self.debug.node_stats_updating_delay
    }

    pub fn get_listening_address(&self) -> IpAddr {
        self.mixnode.listening_address
    }

    pub fn get_announce_address(&self) -> String {
        self.mixnode.announce_address.clone()
    }

    pub fn get_mix_port(&self) -> u16 {
        self.mixnode.mix_port
    }

    pub fn get_verloc_port(&self) -> u16 {
        self.mixnode.verloc_port
    }

    pub fn get_http_api_port(&self) -> u16 {
        self.mixnode.http_api_port
    }

    pub fn get_packet_forwarding_initial_backoff(&self) -> Duration {
        self.debug.packet_forwarding_initial_backoff
    }

    pub fn get_packet_forwarding_maximum_backoff(&self) -> Duration {
        self.debug.packet_forwarding_maximum_backoff
    }

    pub fn get_initial_connection_timeout(&self) -> Duration {
        self.debug.initial_connection_timeout
    }

    pub fn get_maximum_connection_buffer_size(&self) -> usize {
        self.debug.maximum_connection_buffer_size
    }

    pub fn get_version(&self) -> &str {
        &self.mixnode.version
    }

    pub fn get_measurement_packets_per_node(&self) -> usize {
        self.verloc.packets_per_node
    }

    pub fn get_measurement_packet_timeout(&self) -> Duration {
        self.verloc.packet_timeout
    }

    pub fn get_measurement_connection_timeout(&self) -> Duration {
        self.verloc.connection_timeout
    }

    pub fn get_measurement_delay_between_packets(&self) -> Duration {
        self.verloc.delay_between_packets
    }

    pub fn get_measurement_tested_nodes_batch_size(&self) -> usize {
        self.verloc.tested_nodes_batch_size
    }

    pub fn get_measurement_testing_interval(&self) -> Duration {
        self.verloc.testing_interval
    }

    pub fn get_measurement_retry_timeout(&self) -> Duration {
        self.verloc.retry_timeout
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct MixNode {
    /// Version of the mixnode for which this configuration was created.
    #[serde(default = "missing_string_value")]
    version: String,

    /// ID specifies the human readable ID of this particular mixnode.
    id: String,

    /// Address to which this mixnode will bind to and will be listening for packets.
    #[serde(deserialize_with = "de_ipaddr_from_maybe_str_socks_addr")]
    listening_address: IpAddr,

    /// Optional address announced to the validator for the clients to connect to.
    /// It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
    /// later on by using name resolvable with a DNS query, such as `nymtech.net`.
    announce_address: String,

    /// Port used for listening for all mixnet traffic.
    /// (default: 1789)
    #[serde(default = "default_mix_port")]
    mix_port: u16,

    /// Port used for listening for verloc traffic.
    /// (default: 1790)
    #[serde(default = "default_verloc_port")]
    verloc_port: u16,

    /// Port used for listening for http requests.
    /// (default: 8000)
    #[serde(default = "default_http_api_port")]
    http_api_port: u16,

    /// Path to file containing private identity key.
    #[serde(default = "missing_string_value")]
    private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    #[serde(default = "missing_string_value")]
    public_identity_key_file: PathBuf,

    /// Path to file containing private sphinx key.
    private_sphinx_key_file: PathBuf,

    /// Path to file containing public sphinx key.
    public_sphinx_key_file: PathBuf,

    /// Addresses to APIs running on validator from which the node gets the view of the network.
    validator_api_urls: Vec<Url>,

    /// nym_home_directory specifies absolute path to the home nym MixNodes directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl MixNode {
    fn default_private_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("private_identity.pem")
    }

    fn default_public_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("public_identity.pem")
    }

    fn default_private_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("private_sphinx.pem")
    }

    fn default_public_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(Some(id)).join("public_sphinx.pem")
    }
}

impl Default for MixNode {
    fn default() -> Self {
        MixNode {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            listening_address: bind_all_address(),
            announce_address: "127.0.0.1".to_string(),
            mix_port: DEFAULT_MIX_LISTENING_PORT,
            verloc_port: DEFAULT_VERLOC_LISTENING_PORT,
            http_api_port: DEFAULT_HTTP_API_LISTENING_PORT,
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_sphinx_key_file: Default::default(),
            public_sphinx_key_file: Default::default(),
            validator_api_urls: default_api_endpoints(),
            nym_root_directory: Config::default_root_directory(),
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Logging {}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Verloc {
    /// Specifies number of echo packets sent to each node during a measurement run.
    packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the connection to get established.
    connection_timeout: Duration,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    packet_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    retry_timeout: Duration,
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
    node_stats_logging_delay: Duration,

    /// Delay between each subsequent node statistics being updated
    #[serde(with = "humantime_serde")]
    node_stats_updating_delay: Duration,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(with = "humantime_serde")]
    packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    #[serde(with = "humantime_serde")]
    initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    maximum_connection_buffer_size: usize,
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
        }
    }
}
