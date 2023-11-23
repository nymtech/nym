// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config::persistence::{
    CoconutSignerPaths, NetworkMonitorPaths, NodeStatusAPIPaths,
};
use crate::support::config::{
    Base, CirculatingSupplyCacher, CirculatingSupplyCacherDebug, CoconutSigner, CoconutSignerDebug,
    Config, NetworkMonitor, NetworkMonitorDebug, NodeStatusAPI, NodeStatusAPIDebug, Rewarding,
    RewardingDebug, TopologyCacher, TopologyCacherDebug,
};
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use nym_validator_client::nyxd;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

const DEFAULT_NYM_API_PORT: u16 = 8080;
const MIXNET_CONTRACT_ADDRESS: &str =
    "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr";
const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";

const DEFAULT_LOCAL_VALIDATOR: &str = "http://localhost:26657";

const DEFAULT_DKG_CONTRACT_POLLING_RATE: Duration = Duration::from_secs(10);

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
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_21 {
    #[serde(default)]
    base: BaseV1_1_21,

    #[serde(default)]
    network_monitor: NetworkMonitorV1_1_21,

    #[serde(default)]
    node_status_api: NodeStatusAPIV1_1_21,

    #[serde(default)]
    topology_cacher: TopologyCacherV1_1_21,

    #[serde(default)]
    circulating_supply_cacher: CirculatingSupplyCacherV1_1_21,

    #[serde(default)]
    rewarding: RewardingV1_1_21,

    #[serde(default)]
    coconut_signer: CoconutSignerV1_1_21,
}

impl From<ConfigV1_1_21> for Config {
    fn from(value: ConfigV1_1_21) -> Self {
        // this value was never properly saved (probably a bug)
        // so explicitly set it to the default

        let dkg_persistent_state_path =
            CoconutSignerV1_1_21::default_dkg_persistent_state_path(&value.base.id);

        Config {
            base: Base {
                id: value.base.id,
                local_validator: value.base.local_validator,
                mixnet_contract_address: value.base.mixnet_contract_address,
                vesting_contract_address: value.base.vesting_contract_address,
                mnemonic: value.base.mnemonic,
            },
            network_monitor: NetworkMonitor {
                enabled: value.network_monitor.enabled,
                storage_paths: NetworkMonitorPaths {
                    credentials_database_path: value.network_monitor.credentials_database_path,
                },
                debug: NetworkMonitorDebug {
                    min_mixnode_reliability: value.network_monitor.min_mixnode_reliability,
                    min_gateway_reliability: value.network_monitor.min_gateway_reliability,
                    disabled_credentials_mode: value.network_monitor.disabled_credentials_mode,
                    run_interval: value.network_monitor.run_interval,
                    gateway_ping_interval: value.network_monitor.gateway_ping_interval,
                    gateway_sending_rate: value.network_monitor.gateway_sending_rate,
                    max_concurrent_gateway_clients: value
                        .network_monitor
                        .max_concurrent_gateway_clients,
                    gateway_response_timeout: value.network_monitor.gateway_response_timeout,
                    gateway_connection_timeout: value.network_monitor.gateway_connection_timeout,
                    packet_delivery_timeout: value.network_monitor.packet_delivery_timeout,
                    test_routes: value.network_monitor.test_routes,
                    minimum_test_routes: value.network_monitor.minimum_test_routes,
                    route_test_packets: value.network_monitor.route_test_packets,
                    per_node_test_packets: value.network_monitor.per_node_test_packets,
                },
            },
            node_status_api: NodeStatusAPI {
                storage_paths: NodeStatusAPIPaths {
                    database_path: value.node_status_api.database_path,
                },
                debug: NodeStatusAPIDebug {
                    caching_interval: value.node_status_api.caching_interval,
                },
            },
            topology_cacher: TopologyCacher {
                debug: TopologyCacherDebug {
                    caching_interval: value.topology_cacher.caching_interval,
                    ..Default::default()
                },
            },
            circulating_supply_cacher: CirculatingSupplyCacher {
                enabled: value.circulating_supply_cacher.enabled,
                debug: CirculatingSupplyCacherDebug {
                    caching_interval: value.circulating_supply_cacher.caching_interval,
                },
            },
            rewarding: Rewarding {
                enabled: value.rewarding.enabled,
                debug: RewardingDebug {
                    minimum_interval_monitor_threshold: value
                        .rewarding
                        .minimum_interval_monitor_threshold,
                },
            },
            coconut_signer: CoconutSigner {
                enabled: value.coconut_signer.enabled,
                announce_address: value.base.announce_address,
                storage_paths: CoconutSignerPaths {
                    dkg_persistent_state_path,
                    verification_key_path: value.coconut_signer.verification_key_path,
                    secret_key_path: value.coconut_signer.secret_key_path,
                    decryption_key_path: value.coconut_signer.decryption_key_path,
                    public_key_with_proof_path: value.coconut_signer.public_key_with_proof_path,
                },
                debug: CoconutSignerDebug {
                    dkg_contract_polling_rate: value.coconut_signer.dkg_contract_polling_rate,
                },
            },
            ephemera: Default::default(),
        }
    }
}

impl MigrationNymConfig for ConfigV1_1_21 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("nym-api")
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct BaseV1_1_21 {
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

impl Default for BaseV1_1_21 {
    fn default() -> Self {
        let default_validator: Url = DEFAULT_LOCAL_VALIDATOR
            .parse()
            .expect("default local validator is malformed!");
        let mut default_announce_address = default_validator.clone();
        default_announce_address
            .set_port(Some(DEFAULT_NYM_API_PORT))
            .expect("default local validator is malformed!");

        BaseV1_1_21 {
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
#[serde(deny_unknown_fields)]
pub struct NetworkMonitorV1_1_21 {
    min_mixnode_reliability: u8, // defaults to 50
    min_gateway_reliability: u8, // defaults to 20
    enabled: bool,
    #[serde(default)]
    disabled_credentials_mode: bool,
    #[serde(with = "humantime_serde")]
    run_interval: Duration,
    #[serde(with = "humantime_serde")]
    gateway_ping_interval: Duration,
    gateway_sending_rate: usize,
    max_concurrent_gateway_clients: usize,
    #[serde(with = "humantime_serde")]
    gateway_response_timeout: Duration,
    #[serde(with = "humantime_serde")]
    gateway_connection_timeout: Duration,
    #[serde(with = "humantime_serde")]
    packet_delivery_timeout: Duration,
    credentials_database_path: PathBuf,
    test_routes: usize,
    minimum_test_routes: usize,
    route_test_packets: usize,
    per_node_test_packets: usize,
}

impl Default for NetworkMonitorV1_1_21 {
    fn default() -> Self {
        NetworkMonitorV1_1_21 {
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
#[serde(deny_unknown_fields)]
pub struct NodeStatusAPIV1_1_21 {
    database_path: PathBuf,
    #[serde(with = "humantime_serde")]
    caching_interval: Duration,
}

impl Default for NodeStatusAPIV1_1_21 {
    fn default() -> Self {
        NodeStatusAPIV1_1_21 {
            database_path: Default::default(),
            caching_interval: DEFAULT_NODE_STATUS_CACHE_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct TopologyCacherV1_1_21 {
    #[serde(with = "humantime_serde")]
    caching_interval: Duration,
}

impl Default for TopologyCacherV1_1_21 {
    fn default() -> Self {
        TopologyCacherV1_1_21 {
            caching_interval: DEFAULT_TOPOLOGY_CACHE_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct CirculatingSupplyCacherV1_1_21 {
    enabled: bool,

    #[serde(with = "humantime_serde")]
    caching_interval: Duration,
}

impl Default for CirculatingSupplyCacherV1_1_21 {
    fn default() -> Self {
        CirculatingSupplyCacherV1_1_21 {
            enabled: true,
            caching_interval: DEFAULT_CIRCULATING_SUPPLY_CACHE_INTERVAL,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct RewardingV1_1_21 {
    enabled: bool,
    minimum_interval_monitor_threshold: u8,
}

impl Default for RewardingV1_1_21 {
    fn default() -> Self {
        RewardingV1_1_21 {
            enabled: false,
            minimum_interval_monitor_threshold: DEFAULT_MONITOR_THRESHOLD,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct CoconutSignerV1_1_21 {
    enabled: bool,
    dkg_persistent_state_path: PathBuf,
    verification_key_path: PathBuf,
    secret_key_path: PathBuf,
    decryption_key_path: PathBuf,
    public_key_with_proof_path: PathBuf,
    dkg_contract_polling_rate: Duration,
}

impl CoconutSignerV1_1_21 {
    pub const DKG_PERSISTENT_STATE_FILE: &'static str = "dkg_persistent_state.json";

    fn default_dkg_persistent_state_path(id: &str) -> PathBuf {
        ConfigV1_1_21::default_data_directory(id).join(Self::DKG_PERSISTENT_STATE_FILE)
    }
}

impl Default for CoconutSignerV1_1_21 {
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
