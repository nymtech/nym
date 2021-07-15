// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::template::config_template;
use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

mod template;

const DEFAULT_VALIDATOR_REST_ENDPOINTS: &[&str] = &[
    "http://testnet-milhon-validator1.nymtech.net:1317",
    "http://testnet-milhon-validator2.nymtech.net:1317",
];
const DEFAULT_MIXNET_CONTRACT: &str = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen";

const DEFAULT_NODE_STATUS_API: &str = "http://localhost:8081";
const DEFAULT_GATEWAY_SENDING_RATE: usize = 500;
const DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS: usize = 50;
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_millis(1_500);
const DEFAULT_GATEWAY_CONNECTION_TIMEOUT: Duration = Duration::from_millis(2_500);

const DEFAULT_CACHE_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    base: Base,

    #[serde(default)]
    network_monitor: NetworkMonitor,

    #[serde(default)]
    node_status_api: NodeStatusAPI,

    #[serde(default)]
    topology_cacher: TopologyCacher,
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
#[serde(deny_unknown_fields)]
pub struct Base {
    // TODO: this will probably be changed very soon to point only to a single endpoint,
    // that will be a local address
    validator_rest_urls: Vec<String>,

    /// Address of the validator contract managing the network
    mixnet_contract_address: String,
}

impl Default for Base {
    fn default() -> Self {
        Base {
            validator_rest_urls: DEFAULT_VALIDATOR_REST_ENDPOINTS
                .iter()
                .map(|&endpoint| endpoint.to_string())
                .collect(),
            mixnet_contract_address: DEFAULT_MIXNET_CONTRACT.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkMonitor {
    /// Specifies whether network monitoring service is enabled in this process.
    enabled: bool,

    /// Specifies whether a detailed report should be printed after each run
    print_detailed_report: bool,

    // I guess in the future this will be deprecated/removed in favour
    // of choosing 'good' network based on current nodes with best behaviour
    /// Location of .json file containing IPv4 'good' network topology
    good_v4_topology_file: PathBuf,

    /// Location of .json file containing IPv6 'good' network topology
    good_v6_topology_file: PathBuf,

    // TODO: another field that will be replaced very soon when node status api is moved
    // to this process
    /// Address of the node status api to submit results to. Most likely it's a local address
    node_status_api_url: String,

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
}

impl NetworkMonitor {
    fn default_good_v4_topology_file() -> PathBuf {
        Config::default_data_directory(None).join("v4-topology.json")
    }

    fn default_good_v6_topology_file() -> PathBuf {
        Config::default_data_directory(None).join("v6-topology.json")
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        NetworkMonitor {
            enabled: false,
            print_detailed_report: false,
            good_v4_topology_file: Self::default_good_v4_topology_file(),
            good_v6_topology_file: Self::default_good_v6_topology_file(),
            node_status_api_url: DEFAULT_NODE_STATUS_API.to_string(),
            gateway_sending_rate: DEFAULT_GATEWAY_SENDING_RATE,
            max_concurrent_gateway_clients: DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS,
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            gateway_connection_timeout: DEFAULT_GATEWAY_CONNECTION_TIMEOUT,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeStatusAPI {
    // does not yet exist
}

impl Default for NodeStatusAPI {
    fn default() -> Self {
        NodeStatusAPI {}
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
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

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    pub fn enabled_network_monitor(mut self, enabled: bool) -> Self {
        self.network_monitor.enabled = enabled;
        self
    }

    pub fn detailed_network_monitor_report(mut self, detailed: bool) -> Self {
        self.network_monitor.print_detailed_report = detailed;
        self
    }

    pub fn with_v4_good_topology<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.network_monitor.good_v4_topology_file = path.as_ref().to_owned();
        self
    }

    pub fn with_v6_good_topology<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.network_monitor.good_v6_topology_file = path.as_ref().to_owned();
        self
    }

    pub fn with_custom_validators(mut self, validators: Vec<String>) -> Self {
        self.base.validator_rest_urls = validators;
        self
    }

    pub fn with_custom_mixnet_contract<S: Into<String>>(mut self, mixnet_contract: S) -> Self {
        self.base.mixnet_contract_address = mixnet_contract.into();
        self
    }

    pub fn with_custom_node_status_api<S: Into<String>>(mut self, node_status_api: S) -> Self {
        self.network_monitor.node_status_api_url = node_status_api.into();
        self
    }

    pub fn with_gateway_sending_rate(mut self, rate: usize) -> Self {
        self.network_monitor.gateway_sending_rate = rate;
        self
    }

    pub fn with_caching_interval(mut self, interval: Duration) -> Self {
        self.topology_cacher.caching_interval = interval;
        self
    }

    pub fn get_network_monitor_enabled(&self) -> bool {
        self.network_monitor.enabled
    }

    pub fn get_detailed_report(&self) -> bool {
        self.network_monitor.print_detailed_report
    }

    pub fn get_v4_good_topology_file(&self) -> PathBuf {
        self.network_monitor.good_v4_topology_file.clone()
    }

    pub fn get_v6_good_topology_file(&self) -> PathBuf {
        self.network_monitor.good_v6_topology_file.clone()
    }

    pub fn get_validators_urls(&self) -> Vec<String> {
        self.base.validator_rest_urls.clone()
    }

    pub fn get_mixnet_contract_address(&self) -> String {
        self.base.mixnet_contract_address.clone()
    }

    pub fn get_node_status_api_url(&self) -> String {
        self.network_monitor.node_status_api_url.clone()
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

    pub fn get_caching_interval(&self) -> Duration {
        self.topology_cacher.caching_interval
    }
}
