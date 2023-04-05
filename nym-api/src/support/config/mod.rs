// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::template::config_template;
use nym_config::defaults::mainnet::{MIXNET_CONTRACT_ADDRESS, VESTING_CONTRACT_ADDRESS};
use nym_config::defaults::DEFAULT_NYM_API_PORT;
use nym_config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;
use nym_validator_client::nyxd;

mod template;

pub const DEFAULT_LOCAL_VALIDATOR: &str = "http://localhost:26657";

pub const DEFAULT_DKG_CONTRACT_POLLING_RATE: Duration = Duration::from_secs(10);

const DEFAULT_GATEWAY_SENDING_RATE: usize = 200;
const DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS: usize = 50;
const DEFAULT_PACKET_DELIVERY_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_MONITOR_RUN_INTERVAL: Duration = Duration::from_secs(15 * 60);
const DEFAULT_GATEWAY_PING_INTERVAL: Duration = Duration::from_secs(60);
// Set this to a high value for now, so that we don't risk sporadic timeouts that might cause
// bought bandwidth tokens to not have time to be spent; Once we remove the gateway from the
// bandwidth bridging protocol, we can come back to a smaller timeout value
const DEFAULT_GATEWAY_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
// This timeout value should be big enough to accommodate an initial bandwidth acquirement
const DEFAULT_GATEWAY_CONNECTION_TIMEOUT: Duration = Duration::from_secs(2 * 60);

const DEFAULT_TEST_ROUTES: usize = 3;
const DEFAULT_MINIMUM_TEST_ROUTES: usize = 1;
const DEFAULT_ROUTE_TEST_PACKETS: usize = 1000;
const DEFAULT_PER_NODE_TEST_PACKETS: usize = 3;

const DEFAULT_TOPOLOGY_CACHE_INTERVAL: Duration = Duration::from_secs(30);
const DEFAULT_NODE_STATUS_CACHE_INTERVAL: Duration = Duration::from_secs(120);
const DEFAULT_CIRCULATING_SUPPLY_CACHE_INTERVAL: Duration = Duration::from_secs(3600);
const DEFAULT_MONITOR_THRESHOLD: u8 = 60;
const DEFAULT_MIN_MIXNODE_RELIABILITY: u8 = 50;
const DEFAULT_MIN_GATEWAY_RELIABILITY: u8 = 20;

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
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
    circulating_supply_cacher: CirculatingSupplyCacher,

    #[serde(default)]
    rewarding: Rewarding,

    #[serde(default)]
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
            .join("nym-api")
    }

    fn try_default_root_directory() -> Option<PathBuf> {
        dirs::home_dir().map(|path| path.join(".nym").join("nym-api"))
    }

    fn root_directory(&self) -> PathBuf {
        Self::default_root_directory()
    }

    fn config_directory(&self) -> PathBuf {
        self.root_directory().join(self.get_id()).join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.root_directory().join(self.get_id()).join("data")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Base {
    /// ID specifies the human readable ID of this particular nym-api.
    id: String,

    local_validator: Url,

    /// Address announced to the directory server for the clients to connect to.
    // It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
    // later on by using name resolvable with a DNS query, such as `nymtech.net`.
    announce_address: Url,

    /// Address of the validator contract managing the network
    mixnet_contract_address: nyxd::AccountId,

    /// Address of the vesting contract holding locked tokens
    vesting_contract_address: nyxd::AccountId,

    /// Mnemonic used for rewarding and/or multisig operations
    mnemonic: bip39::Mnemonic,
}

impl Default for Base {
    fn default() -> Self {
        let default_validator: Url = DEFAULT_LOCAL_VALIDATOR
            .parse()
            .expect("default local validator is malformed!");
        let mut default_announce_address = default_validator.clone();
        default_announce_address
            .set_port(Some(DEFAULT_NYM_API_PORT))
            .expect("default local validator is malformed!");

        Base {
            id: String::default(),
            local_validator: default_validator,
            announce_address: default_announce_address,
            mixnet_contract_address: MIXNET_CONTRACT_ADDRESS.parse().unwrap(),
            vesting_contract_address: VESTING_CONTRACT_ADDRESS.parse().unwrap(),
            mnemonic: bip39::Mnemonic::generate(24).unwrap(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NetworkMonitor {
    //  Mixnodes and gateways with relialability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    min_mixnode_reliability: u8, // defaults to 50
    min_gateway_reliability: u8, // defaults to 20
    /// Specifies whether network monitoring service is enabled in this process.
    enabled: bool,

    /// Indicates whether this validator api is running in a disabled credentials mode, thus attempting
    /// to claim bandwidth without presenting bandwidth credentials.
    #[serde(default)]
    disabled_credentials_mode: bool,

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
    pub const DB_FILE: &'static str = "credentials_database.db";

    fn default_credentials_database_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join(Self::DB_FILE)
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        NetworkMonitor {
            min_mixnode_reliability: DEFAULT_MIN_MIXNODE_RELIABILITY,
            min_gateway_reliability: DEFAULT_MIN_GATEWAY_RELIABILITY,
            enabled: false,
            disabled_credentials_mode: true,
            run_interval: DEFAULT_MONITOR_RUN_INTERVAL,
            gateway_ping_interval: DEFAULT_GATEWAY_PING_INTERVAL,
            gateway_sending_rate: DEFAULT_GATEWAY_SENDING_RATE,
            max_concurrent_gateway_clients: DEFAULT_MAX_CONCURRENT_GATEWAY_CLIENTS,
            gateway_response_timeout: DEFAULT_GATEWAY_RESPONSE_TIMEOUT,
            gateway_connection_timeout: DEFAULT_GATEWAY_CONNECTION_TIMEOUT,
            packet_delivery_timeout: DEFAULT_PACKET_DELIVERY_TIMEOUT,
            credentials_database_path: Default::default(),
            test_routes: DEFAULT_TEST_ROUTES,
            minimum_test_routes: DEFAULT_MINIMUM_TEST_ROUTES,
            route_test_packets: DEFAULT_ROUTE_TEST_PACKETS,
            per_node_test_packets: DEFAULT_PER_NODE_TEST_PACKETS,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NodeStatusAPI {
    /// Path to the database file containing uptime statuses for all mixnodes and gateways.
    database_path: PathBuf,

    #[serde(with = "humantime_serde")]
    caching_interval: Duration,
}

impl NodeStatusAPI {
    pub const DB_FILE: &'static str = "db.sqlite";

    fn default_database_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join(Self::DB_FILE)
    }
}

impl Default for NodeStatusAPI {
    fn default() -> Self {
        NodeStatusAPI {
            database_path: Default::default(),
            caching_interval: DEFAULT_NODE_STATUS_CACHE_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct TopologyCacher {
    #[serde(with = "humantime_serde")]
    caching_interval: Duration,
}

impl Default for TopologyCacher {
    fn default() -> Self {
        TopologyCacher {
            caching_interval: DEFAULT_TOPOLOGY_CACHE_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct CirculatingSupplyCacher {
    enabled: bool,

    #[serde(with = "humantime_serde")]
    caching_interval: Duration,
}

impl Default for CirculatingSupplyCacher {
    fn default() -> Self {
        CirculatingSupplyCacher {
            enabled: true,
            caching_interval: DEFAULT_CIRCULATING_SUPPLY_CACHE_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct CoconutSigner {
    /// Specifies whether rewarding service is enabled in this process.
    enabled: bool,

    /// Path to a JSON file where state is persisted between different stages of DKG.
    dkg_persistent_state_path: PathBuf,

    /// Path to the coconut verification key.
    verification_key_path: PathBuf,

    /// Path to the coconut secret key.
    secret_key_path: PathBuf,

    /// Path to the dkg dealer decryption key.
    decryption_key_path: PathBuf,

    /// Path to the dkg dealer public key with proof.
    public_key_with_proof_path: PathBuf,

    /// Duration of the interval for polling the dkg contract.
    dkg_contract_polling_rate: Duration,
}

impl CoconutSigner {
    pub const DKG_PERSISTENT_STATE_FILE: &'static str = "dkg_persistent_state.json";
    pub const DKG_DECRYPTION_KEY_FILE: &'static str = "dkg_decryption_key.pem";
    pub const DKG_PUBLIC_KEY_WITH_PROOF_FILE: &'static str = "dkg_public_key_with_proof.pem";
    pub const COCONUT_VERIFICATION_KEY_FILE: &'static str = "coconut_verification_key.pem";
    pub const COCONUT_SECRET_KEY_FILE: &'static str = "coconut_secret_key.pem";

    fn default_coconut_verification_key_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join(Self::COCONUT_VERIFICATION_KEY_FILE)
    }

    fn default_coconut_secret_key_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join(Self::COCONUT_SECRET_KEY_FILE)
    }

    fn default_dkg_persistent_state_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join(Self::DKG_PERSISTENT_STATE_FILE)
    }

    fn default_dkg_decryption_key_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join(Self::DKG_DECRYPTION_KEY_FILE)
    }

    fn default_dkg_public_key_with_proof_path(id: &str) -> PathBuf {
        Config::default_data_directory(id).join(Self::DKG_PUBLIC_KEY_WITH_PROOF_FILE)
    }
}

impl Default for CoconutSigner {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            dkg_persistent_state_path: Default::default(),
            verification_key_path: Default::default(),
            secret_key_path: Default::default(),
            decryption_key_path: Default::default(),
            public_key_with_proof_path: Default::default(),
            dkg_contract_polling_rate: DEFAULT_DKG_CONTRACT_POLLING_RATE,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.base.id = id.to_string();
        self.node_status_api.database_path = NodeStatusAPI::default_database_path(id);
        self.network_monitor.credentials_database_path =
            NetworkMonitor::default_credentials_database_path(id);
        self.coconut_signer.dkg_persistent_state_path =
            CoconutSigner::default_dkg_persistent_state_path(id);
        self.coconut_signer.verification_key_path =
            CoconutSigner::default_coconut_verification_key_path(id);
        self.coconut_signer.secret_key_path = CoconutSigner::default_coconut_secret_key_path(id);
        self.coconut_signer.decryption_key_path =
            CoconutSigner::default_dkg_decryption_key_path(id);
        self.coconut_signer.public_key_with_proof_path =
            CoconutSigner::default_dkg_public_key_with_proof_path(id);
        self
    }

    pub fn with_network_monitor_enabled(mut self, enabled: bool) -> Self {
        self.network_monitor.enabled = enabled;
        self
    }

    pub fn with_disabled_credentials_mode(mut self, disabled_credentials_mode: bool) -> Self {
        self.network_monitor.disabled_credentials_mode = disabled_credentials_mode;
        self
    }

    pub fn with_rewarding_enabled(mut self, enabled: bool) -> Self {
        self.rewarding.enabled = enabled;
        self
    }

    pub fn with_coconut_signer_enabled(mut self, enabled: bool) -> Self {
        self.coconut_signer.enabled = enabled;
        self
    }

    pub fn with_custom_nyxd_validator(mut self, validator: Url) -> Self {
        self.base.local_validator = validator;
        self
    }

    pub fn with_announce_address(mut self, announce_address: Url) -> Self {
        self.base.announce_address = announce_address;
        self
    }

    pub fn with_custom_mixnet_contract(mut self, mixnet_contract: nyxd::AccountId) -> Self {
        self.base.mixnet_contract_address = mixnet_contract;
        self
    }

    pub fn with_custom_vesting_contract(mut self, vesting_contract: nyxd::AccountId) -> Self {
        self.base.vesting_contract_address = vesting_contract;
        self
    }

    pub fn with_mnemonic(mut self, mnemonic: bip39::Mnemonic) -> Self {
        self.base.mnemonic = mnemonic;
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

    pub fn get_id(&self) -> String {
        self.base.id.clone()
    }

    pub fn get_network_monitor_enabled(&self) -> bool {
        self.network_monitor.enabled
    }

    pub fn get_coconut_signer_enabled(&self) -> bool {
        self.coconut_signer.enabled
    }

    pub fn get_disabled_credentials_mode(&self) -> bool {
        self.network_monitor.disabled_credentials_mode
    }

    pub fn get_credentials_database_path(&self) -> PathBuf {
        self.network_monitor.credentials_database_path.clone()
    }

    pub fn get_rewarding_enabled(&self) -> bool {
        self.rewarding.enabled
    }

    pub fn get_nyxd_url(&self) -> Url {
        self.base.local_validator.clone()
    }

    pub fn get_announce_address(&self) -> Url {
        self.base.announce_address.clone()
    }

    pub fn get_mixnet_contract_address(&self) -> nyxd::AccountId {
        self.base.mixnet_contract_address.clone()
    }

    pub fn get_vesting_contract_address(&self) -> nyxd::AccountId {
        self.base.vesting_contract_address.clone()
    }

    pub fn get_mnemonic(&self) -> bip39::Mnemonic {
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

    pub fn get_topology_caching_interval(&self) -> Duration {
        self.topology_cacher.caching_interval
    }

    pub fn get_node_status_caching_interval(&self) -> Duration {
        self.node_status_api.caching_interval
    }

    pub fn get_circulating_supply_caching_interval(&self) -> Duration {
        self.circulating_supply_cacher.caching_interval
    }

    pub fn get_circulating_supply_enabled(&self) -> bool {
        self.circulating_supply_cacher.enabled
    }

    pub fn get_node_status_api_database_path(&self) -> PathBuf {
        self.node_status_api.database_path.clone()
    }

    pub fn persistent_state_path(&self) -> PathBuf {
        self.coconut_signer.dkg_persistent_state_path.clone()
    }

    pub fn verification_key_path(&self) -> PathBuf {
        self.coconut_signer.verification_key_path.clone()
    }

    pub fn secret_key_path(&self) -> PathBuf {
        self.coconut_signer.secret_key_path.clone()
    }

    pub fn decryption_key_path(&self) -> PathBuf {
        self.coconut_signer.decryption_key_path.clone()
    }

    pub fn public_key_with_proof_path(&self) -> PathBuf {
        self.coconut_signer.public_key_with_proof_path.clone()
    }

    pub fn get_dkg_contract_polling_rate(&self) -> Duration {
        self.coconut_signer.dkg_contract_polling_rate
    }

    // TODO: Remove if still unused
    #[allow(dead_code)]
    pub fn get_minimum_interval_monitor_threshold(&self) -> u8 {
        self.rewarding.minimum_interval_monitor_threshold
    }
}
