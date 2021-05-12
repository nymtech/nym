// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::config_template;
use config::{deserialize_duration, deserialize_validators, NymConfig};
use log::error;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

pub mod persistence;
mod template;

pub(crate) const MISSING_VALUE: &str = "MISSING VALUE";

// 'MIXNODE'
const DEFAULT_LISTENING_PORT: u16 = 1789;
pub(crate) const DEFAULT_VALIDATOR_REST_ENDPOINTS: &[&str] = &[
    "http://testnet-finney-validator.nymtech.net:1317",
    "http://testnet-finney-validator2.nymtech.net:1317",
    "http://mixnet.club:1317",
];
pub(crate) const DEFAULT_METRICS_SERVER: &str = "http://testnet-metrics.nymtech.net:8080";
pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "hal1k0jntykt7e4g3y88ltc60czgjuqdy4c9c6gv94";

// 'RTT MEASUREMENT'
const DEFAULT_PACKETS_PER_NODE: usize = 100;
const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
const DEFAULT_BATCH_SIZE: usize = 50;
const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);

// 'DEBUG'
const DEFAULT_METRICS_RUNNING_STATS_LOGGING_DELAY: Duration = Duration::from_millis(60_000);
const DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF: Duration = Duration::from_millis(10_000);
const DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF: Duration = Duration::from_millis(300_000);
const DEFAULT_INITIAL_CONNECTION_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_CACHE_ENTRY_TTL: Duration = Duration::from_millis(30_000);
const DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE: usize = 128;

// helper function to get default validators as a Vec<String>
pub fn default_validator_rest_endpoints() -> Vec<String> {
    DEFAULT_VALIDATOR_REST_ENDPOINTS
        .iter()
        .map(|&endpoint| endpoint.to_string())
        .collect()
}

pub fn missing_string_value<T: From<String>>() -> T {
    MISSING_VALUE.to_string().into()
}

pub fn missing_vec_string_value() -> Vec<String> {
    vec![missing_string_value()]
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    mixnode: MixNode,

    #[serde(default)]
    rtt_measurement: RttMeasurement,
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

    pub fn with_layer(mut self, layer: u64) -> Self {
        self.mixnode.layer = layer;
        self
    }

    pub fn with_custom_validators(mut self, validators: Vec<String>) -> Self {
        self.mixnode.validator_rest_urls = validators;
        self
    }

    pub fn with_custom_mixnet_contract<S: Into<String>>(mut self, mixnet_contract: S) -> Self {
        self.mixnode.mixnet_contract_address = mixnet_contract.into();
        self
    }

    pub fn with_custom_metrics_server<S: Into<String>>(mut self, server: S) -> Self {
        self.mixnode.metrics_server_url = server.into();
        self
    }

    pub fn with_listening_host<S: Into<String>>(mut self, host: S) -> Self {
        // see if the provided `host` is just an ip address or ip:port
        let host = host.into();

        // is it ip:port?
        match SocketAddr::from_str(host.as_ref()) {
            Ok(socket_addr) => {
                self.mixnode.listening_address = socket_addr;
                self
            }
            // try just for ip
            Err(_) => match IpAddr::from_str(host.as_ref()) {
                Ok(ip_addr) => {
                    self.mixnode.listening_address.set_ip(ip_addr);
                    self
                }
                Err(_) => {
                    error!(
                        "failed to make any changes to config - invalid host {}",
                        host
                    );
                    self
                }
            },
        }
    }

    pub fn with_listening_port(mut self, port: u16) -> Self {
        self.mixnode.listening_address.set_port(port);
        self
    }

    pub fn with_announce_host<S: Into<String>>(mut self, host: S) -> Self {
        // this is slightly more complicated as we store announce information as String,
        // since it might not necessarily be a valid SocketAddr (say `nymtech.net:8080` is a valid
        // announce address, yet invalid SocketAddr`

        // first lets see if we received host:port or just host part of an address
        let host = host.into();
        match host.split(':').count() {
            1 => {
                // we provided only 'host' part so we are going to reuse existing port
                self.mixnode.announce_address =
                    format!("{}:{}", host, self.mixnode.listening_address.port());
                self
            }
            2 => {
                // we provided 'host:port' so just put the whole thing there
                self.mixnode.announce_address = host;
                self
            }
            _ => {
                // we provided something completely invalid, so don't try to parse it
                error!(
                    "failed to make any changes to config - invalid announce host {}",
                    host
                );
                self
            }
        }
    }

    pub fn announce_host_from_listening_host(mut self) -> Self {
        self.mixnode.announce_address = self.mixnode.listening_address.to_string();
        self
    }

    pub fn with_announce_port(mut self, port: u16) -> Self {
        let current_host: Vec<_> = self.mixnode.announce_address.split(':').collect();
        debug_assert_eq!(current_host.len(), 2);
        self.mixnode.announce_address = format!("{}:{}", current_host[0], port);
        self
    }

    pub fn with_custom_version(mut self, version: &str) -> Self {
        self.mixnode.version = version.to_string();
        self
    }

    // getters
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

    pub fn get_validator_rest_endpoints(&self) -> Vec<String> {
        self.mixnode.validator_rest_urls.clone()
    }

    pub fn get_validator_mixnet_contract_address(&self) -> String {
        self.mixnode.mixnet_contract_address.clone()
    }

    pub fn get_metrics_server(&self) -> String {
        self.mixnode.metrics_server_url.clone()
    }

    pub fn get_metrics_running_stats_logging_delay(&self) -> Duration {
        self.debug.metrics_running_stats_logging_delay
    }

    pub fn get_layer(&self) -> u64 {
        self.mixnode.layer
    }

    pub fn get_listening_address(&self) -> SocketAddr {
        self.mixnode.listening_address
    }

    pub fn get_announce_address(&self) -> String {
        self.mixnode.announce_address.clone()
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

    pub fn get_cache_entry_ttl(&self) -> Duration {
        self.debug.cache_entry_ttl
    }

    pub fn get_version(&self) -> &str {
        &self.mixnode.version
    }

    pub fn get_measurement_packets_per_node(&self) -> usize {
        self.rtt_measurement.packets_per_node
    }
    pub fn get_measurement_packet_timeout(&self) -> Duration {
        self.rtt_measurement.packet_timeout
    }
    pub fn get_measurement_delay_between_packets(&self) -> Duration {
        self.rtt_measurement.delay_between_packets
    }
    pub fn get_measurement_tested_nodes_batch_size(&self) -> usize {
        self.rtt_measurement.tested_nodes_batch_size
    }
    pub fn get_measurement_testing_interval(&self) -> Duration {
        self.rtt_measurement.testing_interval
    }
    pub fn get_measurement_retry_timeout(&self) -> Duration {
        self.rtt_measurement.retry_timeout
    }

    // upgrade-specific
    pub(crate) fn set_default_identity_keypair_paths(&mut self) {
        self.mixnode.private_identity_key_file =
            self::MixNode::default_private_identity_key_file(&self.mixnode.id);
        self.mixnode.public_identity_key_file =
            self::MixNode::default_public_identity_key_file(&self.mixnode.id);
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct MixNode {
    /// Version of the mixnode for which this configuration was created.
    #[serde(default = "missing_string_value")]
    version: String,

    /// ID specifies the human readable ID of this particular mixnode.
    id: String,

    /// Layer of this particular mixnode determining its position in the network.
    layer: u64,

    /// Socket address to which this mixnode will bind to and will be listening for packets.
    listening_address: SocketAddr,

    /// Optional address announced to the validator for the clients to connect to.
    /// It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
    /// later on by using name resolvable with a DNS query, such as `nymtech.net:8080`.
    /// Additionally a custom port can be provided, so both `nymtech.net:8080` and `nymtech.net`
    /// are valid announce addresses, while the later will default to whatever port is used for
    /// `listening_address`.
    announce_address: String,

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

    /// Validator server to which the node will be reporting their presence data.
    #[serde(
        deserialize_with = "deserialize_validators",
        default = "missing_vec_string_value",
        alias = "validator_rest_url"
    )]
    validator_rest_urls: Vec<String>,

    /// Address of the validator contract managing the network.
    #[serde(default = "missing_string_value")]
    mixnet_contract_address: String,

    /// Metrics server to which the node will be reporting their metrics data.
    #[serde(default = "missing_string_value")]
    metrics_server_url: String,

    /// nym_home_directory specifies absolute path to the home nym MixNodes directory.
    /// It is expected to use default value and hence .toml file should not redefine this field.
    nym_root_directory: PathBuf,
}

impl MixNode {
    fn default_private_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("private_identity.pem")
    }

    fn default_public_identity_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("public_identity.pem")
    }

    fn default_private_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("private_sphinx.pem")
    }

    fn default_public_sphinx_key_file(id: &str) -> PathBuf {
        Config::default_data_directory(id).join("public_sphinx.pem")
    }
}

impl Default for MixNode {
    fn default() -> Self {
        MixNode {
            version: env!("CARGO_PKG_VERSION").to_string(),
            id: "".to_string(),
            layer: 0,
            listening_address: format!("0.0.0.0:{}", DEFAULT_LISTENING_PORT)
                .parse()
                .unwrap(),
            announce_address: format!("127.0.0.1:{}", DEFAULT_LISTENING_PORT),
            private_identity_key_file: Default::default(),
            public_identity_key_file: Default::default(),
            private_sphinx_key_file: Default::default(),
            public_sphinx_key_file: Default::default(),
            validator_rest_urls: default_validator_rest_endpoints(),
            mixnet_contract_address: DEFAULT_MIXNET_CONTRACT_ADDRESS.to_string(),
            metrics_server_url: DEFAULT_METRICS_SERVER.to_string(),
            nym_root_directory: Config::default_root_directory(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Logging {}

impl Default for Logging {
    fn default() -> Self {
        Logging {}
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RttMeasurement {
    /// Specifies number of echo packets sent to each node during a measurement run.
    packets_per_node: usize,

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

impl Default for RttMeasurement {
    fn default() -> Self {
        RttMeasurement {
            packets_per_node: DEFAULT_PACKETS_PER_NODE,
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
    /// Delay between each subsequent running metrics statistics being logged.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    metrics_running_stats_logging_delay: Duration,

    /// Initial value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    packet_forwarding_initial_backoff: Duration,

    /// Maximum value of an exponential backoff to reconnect to dropped TCP connection when
    /// forwarding sphinx packets.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    packet_forwarding_maximum_backoff: Duration,

    /// Timeout for establishing initial connection when trying to forward a sphinx packet.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    initial_connection_timeout: Duration,

    /// Maximum number of packets that can be stored waiting to get sent to a particular connection.
    maximum_connection_buffer_size: usize,

    /// Duration for which a cached vpn processing result is going to get stored for.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "humantime_serde::serialize"
    )]
    cache_entry_ttl: Duration,
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            metrics_running_stats_logging_delay: DEFAULT_METRICS_RUNNING_STATS_LOGGING_DELAY,
            packet_forwarding_initial_backoff: DEFAULT_PACKET_FORWARDING_INITIAL_BACKOFF,
            packet_forwarding_maximum_backoff: DEFAULT_PACKET_FORWARDING_MAXIMUM_BACKOFF,
            initial_connection_timeout: DEFAULT_INITIAL_CONNECTION_TIMEOUT,
            maximum_connection_buffer_size: DEFAULT_MAXIMUM_CONNECTION_BUFFER_SIZE,
            cache_entry_ttl: DEFAULT_CACHE_ENTRY_TTL,
        }
    }
}
