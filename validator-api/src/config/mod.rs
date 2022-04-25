// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::config_template;
use config::defaults::{default_api_endpoints, DEFAULT_NETWORK};
use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

#[cfg(feature = "coconut")]
use coconut_interface::{Base58, KeyPair};

mod template;

pub const DEFAULT_LOCAL_VALIDATOR: &str = "http://localhost:26657";

const DEFAULT_GATEWAY_SENDING_RATE: usize = 200;
const DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS: usize = 50;
const DEFAULT_PACKET_DELIVERY_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_MONITOR_RUN_INTERVAL: Duration = Duration::from_secs(15 * 60);
const DEFAULT_GATEWAY_PING_INTERVAL: Duration = Duration::from_secs(60);
// Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
// bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
// bandwidth bridging protocol, we can come back to a smaller timeout value
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const DEFAULT_GATEWAY_CONNECTION_TIMEOUT: Duration = Duration::from_millis(2_500);

#[cfg(not(feature = "coconut"))]
const DEFAULT_ETH_ENDPOINT: &str = "https://rinkeby.infura.io/v3/00000000000000000000000000000000";
#[cfg(not(feature = "coconut"))]
const DEFAULT_ETH_PRIVATE_KEY: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";

const DEFAULT_TEST_ROUTES: usize = 3;
const DEFAULT_MINIMUM_TEST_ROUTES: usize = 1;
const DEFAULT_ROUTE_TEST_PACKETS: usize = 1000;
const DEFAULT_PER_NODE_TEST_PACKETS: usize = 3;

const DEFAULT_CACHE_INTERVAL: Duration = Duration::from_secs(30);
const DEFAULT_MONITOR_THRESHOLD: u8 = 60;
const DEFAULT_MIN_MIXNODE_RELIABILITY: u8 = 50;
const DEFAULT_MIN_GATEWAY_RELIABILITY: u8 = 20;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Config {
    #[serde(default)]
    base: Base,

    #[serde(default)]
    network_monitor: NetworkMonitor,

    #[serde(default)]
    node_status_api: NodeStatusAPI,

    #[serde(default)]
    topology_cacher: TopologyCacher,

    #[serde(default)]
    rewarding: Rewarding,

    #[serde(default)]
    #[cfg(feature = "coconut")]
    coconut_signer: CoconutSigner,
}

impl NymConfig for Config {
    fn template() -> &'static str {
        config_template()
    }

    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("validator-api")
    }

    fn root_directory(&self) -> PathBuf {
        Self::default_root_directory()
    }

    fn config_directory(&self) -> PathBuf {
        self.root_directory().join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.root_directory().join("data")
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Base {
    local_validator: Url,

    /// Address of the validator contract managing the network
    mixnet_contract_address: String,

    /// Mnemonic used for rewarding and/or multisig operations
    mnemonic: String,
}

impl Default for Base {
    fn default() -> Self {
        Base {
            local_validator: DEFAULT_LOCAL_VALIDATOR
                .parse()
                .expect("default local validator is malformed!"),
            mixnet_contract_address: DEFAULT_NETWORK.mixnet_contract_address().to_string(),
            mnemonic: String::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NetworkMonitor {
    //  Mixnodes and gateways with relialability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    min_mixnode_reliability: u8, // defaults to 50
    min_gateway_reliability: u8, // defaults to 20
    /// Specifies whether network monitoring service is enabled in this process.
    enabled: bool,

    /// Indicates whether this validator api is running in a testnet mode, thus attempting
    /// to claim bandwidth without presenting bandwidth credentials.
    #[serde(default)]
    testnet_mode: bool,

    /// Specifies list of all validators on the network issuing coconut credentials.
    /// A special care must be taken to ensure they are in correct order.
    /// The list must also contain THIS validator that is running the test
    all_validator_apis: Vec<Url>,

    /// Specifies the interval at which the network monitor sends the test packets.
    #[serde(with = "humantime_serde")]
    run_interval: Duration,

    /// Specifies interval at which we should be sending ping packets to all active gateways
    /// in order to keep the websocket connections alive.
    #[serde(with = "humantime_serde")]
    gateway_ping_interval: Duration,

    /// Specifies maximum rate (in packets per second) of test packets being sent to gateway
    gateway_sending_rate: usize,

    /// Maximum number of gateway clients the network monitor will try to talk to concurrently.
    /// 0 = no limit
    max_concurrent_gateway_clients: usize,

    /// Maximum allowed time for receiving gateway response.
    #[serde(with = "humantime_serde")]
    gateway_response_timeout: Duration,

    /// Maximum allowed time for the gateway connection to get established.
    #[serde(with = "humantime_serde")]
    gateway_connection_timeout: Duration,

    /// Specifies the duration the monitor is going to wait after sending all measurement
    /// packets before declaring nodes unreachable.
    #[serde(with = "humantime_serde")]
    packet_delivery_timeout: Duration,

    /// Path to the database containing bandwidth credentials of this client.
    credentials_database_path: PathBuf,

    /// Ethereum private key.
    #[cfg(not(feature = "coconut"))]
    eth_private_key: String,

    /// Addess to an Ethereum full node.
    #[cfg(not(feature = "coconut"))]
    eth_endpoint: String,

    /// Desired number of test routes to be constructed (and working) during a monitor test run.
    test_routes: usize,

    /// The minimum number of test routes that need to be constructed (and working) in order for
    /// a monitor test run to be valid.
    minimum_test_routes: usize,

    /// Number of test packets sent via each pseudorandom route to verify whether they work correctly,
    /// before using them for testing the rest of the network.
    route_test_packets: usize,

    /// Number of test packets sent to each node during regular monitor test run.
    per_node_test_packets: usize,
}

impl NetworkMonitor {
    fn default_credentials_database_path() -> PathBuf {
        Config::default_data_directory(None).join("credentials_database.db")
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        NetworkMonitor {
            min_mixnode_reliability: DEFAULT_MIN_MIXNODE_RELIABILITY,
            min_gateway_reliability: DEFAULT_MIN_GATEWAY_RELIABILITY,
            enabled: false,
            testnet_mode: false,
            all_validator_apis: default_api_endpoints(),
            run_interval: DEFAULT_MONITOR_RUN_INTERVAL,
            gateway_ping_interval: DEFAULT_GATEWAY_PING_INTERVAL,
            gateway_sending_rate: DEFAULT_GATEWAY_SENDING_RATE,
            max_concurrent_gateway_clients: DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS,
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            gateway_connection_timeout: DEFAULT_GATEWAY_CONNECTION_TIMEOUT,
            packet_delivery_timeout: DEFAULT_PACKET_DELIVERY_TIMEOUT,
            credentials_database_path: Self::default_credentials_database_path(),
            #[cfg(not(feature = "coconut"))]
            eth_private_key: DEFAULT_ETH_PRIVATE_KEY.to_string(),
            #[cfg(not(feature = "coconut"))]
            eth_endpoint: DEFAULT_ETH_ENDPOINT.to_string(),
            test_routes: DEFAULT_TEST_ROUTES,
            minimum_test_routes: DEFAULT_MINIMUM_TEST_ROUTES,
            route_test_packets: DEFAULT_ROUTE_TEST_PACKETS,
            per_node_test_packets: DEFAULT_PER_NODE_TEST_PACKETS,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NodeStatusAPI {
    /// Path to the database file containing uptime statuses for all mixnodes and gateways.
    database_path: PathBuf,
}

impl NodeStatusAPI {
    fn default_database_path() -> PathBuf {
        Config::default_data_directory(None).join("db.sqlite")
    }
}

impl Default for NodeStatusAPI {
    fn default() -> Self {
        NodeStatusAPI {
            database_path: Self::default_database_path(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TopologyCacher {
    #[serde(with = "humantime_serde")]
    caching_interval: Duration,
}

impl Default for TopologyCacher {
    fn default() -> Self {
        TopologyCacher {
            caching_interval: DEFAULT_CACHE_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Rewarding {
    /// Specifies whether rewarding service is enabled in this process.
    enabled: bool,

    /// Specifies the minimum percentage of monitor test run data present in order to
    /// distribute rewards for given interval.
    /// Note, only values in range 0-100 are valid
    minimum_interval_monitor_threshold: u8,
}

impl Default for Rewarding {
    fn default() -> Self {
        Rewarding {
            enabled: false,
            minimum_interval_monitor_threshold: DEFAULT_MONITOR_THRESHOLD,
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[cfg(feature = "coconut")]
pub struct CoconutSigner {
    /// Specifies whether rewarding service is enabled in this process.
    enabled: bool,

    /// Base58 encoded signing keypair
    keypair_bs58: String,
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    #[cfg(feature = "coconut")]
    pub fn keypair(&self) -> KeyPair {
        KeyPair::try_from_bs58(self.coconut_signer.keypair_bs58.clone()).unwrap()
    }

    pub fn with_network_monitor_enabled(mut self, enabled: bool) -> Self {
        self.network_monitor.enabled = enabled;
        self
    }

    pub fn with_testnet_mode(mut self, testnet_mode: bool) -> Self {
        self.network_monitor.testnet_mode = testnet_mode;
        self
    }

    pub fn with_rewarding_enabled(mut self, enabled: bool) -> Self {
        self.rewarding.enabled = enabled;
        self
    }

    #[cfg(feature = "coconut")]
    pub fn with_coconut_signer_enabled(mut self, enabled: bool) -> Self {
        self.coconut_signer.enabled = enabled;
        self
    }

    pub fn with_custom_nymd_validator(mut self, validator: Url) -> Self {
        self.base.local_validator = validator;
        self
    }

    pub fn with_custom_mixnet_contract<S: Into<String>>(mut self, mixnet_contract: S) -> Self {
        self.base.mixnet_contract_address = mixnet_contract.into();
        self
    }

    pub fn with_mnemonic<S: Into<String>>(mut self, mnemonic: S) -> Self {
        self.base.mnemonic = mnemonic.into();
        self
    }

    #[cfg(feature = "coconut")]
    pub fn with_keypair<S: Into<String>>(mut self, keypair_bs58: S) -> Self {
        self.coconut_signer.keypair_bs58 = keypair_bs58.into();
        self
    }

    pub fn with_custom_validator_apis(mut self, validator_api_urls: Vec<Url>) -> Self {
        self.network_monitor.all_validator_apis = validator_api_urls;
        self
    }

    pub fn with_minimum_interval_monitor_threshold(mut self, threshold: u8) -> Self {
        self.rewarding.minimum_interval_monitor_threshold = threshold;
        self
    }

    pub fn with_min_mixnode_reliability(mut self, min_mixnode_reliability: u8) -> Self {
        self.network_monitor.min_mixnode_reliability = min_mixnode_reliability;
        self
    }

    pub fn with_min_gateway_reliability(mut self, min_gateway_reliability: u8) -> Self {
        self.network_monitor.min_gateway_reliability = min_gateway_reliability;
        self
    }

    #[cfg(not(feature = "coconut"))]
    pub fn with_eth_private_key(mut self, eth_private_key: String) -> Self {
        self.network_monitor.eth_private_key = eth_private_key;
        self
    }

    #[cfg(not(feature = "coconut"))]
    pub fn with_eth_endpoint(mut self, eth_endpoint: String) -> Self {
        self.network_monitor.eth_endpoint = eth_endpoint;
        self
    }

    pub fn get_network_monitor_enabled(&self) -> bool {
        self.network_monitor.enabled
    }

    #[cfg(feature = "coconut")]
    pub fn get_coconut_signer_enabled(&self) -> bool {
        self.coconut_signer.enabled
    }

    pub fn get_testnet_mode(&self) -> bool {
        self.network_monitor.testnet_mode
    }

    pub fn get_credentials_database_path(&self) -> PathBuf {
        self.network_monitor.credentials_database_path.clone()
    }

    #[cfg(not(feature = "coconut"))]
    pub fn get_network_monitor_eth_private_key(&self) -> String {
        self.network_monitor.eth_private_key.clone()
    }

    #[cfg(not(feature = "coconut"))]
    pub fn get_network_monitor_eth_endpoint(&self) -> String {
        self.network_monitor.eth_endpoint.clone()
    }

    // TODO: Remove if still unused
    #[allow(dead_code)]
    pub fn get_rewarding_enabled(&self) -> bool {
        self.rewarding.enabled
    }

    pub fn get_nymd_validator_url(&self) -> Url {
        self.base.local_validator.clone()
    }

    pub fn get_mixnet_contract_address(&self) -> String {
        self.base.mixnet_contract_address.clone()
    }

    pub fn get_mnemonic(&self) -> String {
        self.base.mnemonic.clone()
    }

    pub fn get_network_monitor_run_interval(&self) -> Duration {
        self.network_monitor.run_interval
    }

    pub fn get_gateway_ping_interval(&self) -> Duration {
        self.network_monitor.gateway_ping_interval
    }

    pub fn get_packet_delivery_timeout(&self) -> Duration {
        self.network_monitor.packet_delivery_timeout
    }

    pub fn get_gateway_sending_rate(&self) -> usize {
        self.network_monitor.gateway_sending_rate
    }

    pub fn get_max_concurrent_gateway_clients(&self) -> usize {
        self.network_monitor.max_concurrent_gateway_clients
    }

    pub fn get_gateway_response_timeout(&self) -> Duration {
        self.network_monitor.gateway_response_timeout
    }

    pub fn get_gateway_connection_timeout(&self) -> Duration {
        self.network_monitor.gateway_connection_timeout
    }

    pub fn get_test_routes(&self) -> usize {
        self.network_monitor.test_routes
    }

    pub fn get_minimum_test_routes(&self) -> usize {
        self.network_monitor.minimum_test_routes
    }

    pub fn get_route_test_packets(&self) -> usize {
        self.network_monitor.route_test_packets
    }

    pub fn get_per_node_test_packets(&self) -> usize {
        self.network_monitor.per_node_test_packets
    }

    pub fn get_caching_interval(&self) -> Duration {
        self.topology_cacher.caching_interval
    }

    pub fn get_node_status_api_database_path(&self) -> PathBuf {
        self.node_status_api.database_path.clone()
    }

    // fix dead code warnings as this method is only ever used with coconut feature
    #[cfg(feature = "coconut")]
    pub fn get_all_validator_api_endpoints(&self) -> Vec<Url> {
        self.network_monitor.all_validator_apis.clone()
    }

    // TODO: Remove if still unused
    #[allow(dead_code)]
    pub fn get_minimum_interval_monitor_threshold(&self) -> u8 {
        self.rewarding.minimum_interval_monitor_threshold
    }
}
