// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::config::persistence::{
    CoconutSignerPaths, NetworkMonitorPaths, NodeStatusAPIPaths,
};
use crate::support::config::{
    Base, CirculatingSupplyCacher, CoconutSigner, CoconutSignerDebug, Config, Ephemera,
    NetworkMonitor, NetworkMonitorDebug, NodeStatusAPI, NodeStatusAPIDebug, Rewarding,
    TopologyCacher,
};
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use nym_validator_client::nyxd;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

const DEFAULT_NYM_API_PORT: u16 = 8080;
const MIXNET_CONTRACT_ADDRESS: &str =
    "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr";
const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";

const DEFAULT_LOCAL_VALIDATOR: &str = "http://localhost:26657";

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_27 {
    #[serde(default)]
    base: BaseV1_1_27,

    #[serde(default)]
    network_monitor: NetworkMonitorV1_1_27,

    #[serde(default)]
    node_status_api: NodeStatusAPIV1_1_27,

    #[serde(default)]
    topology_cacher: TopologyCacher,

    #[serde(default)]
    circulating_supply_cacher: CirculatingSupplyCacher,

    #[serde(default)]
    rewarding: Rewarding,

    #[serde(default)]
    coconut_signer: CoconutSignerV1_1_27,
}

impl From<ConfigV1_1_27> for Config {
    fn from(value: ConfigV1_1_27) -> Self {
        // this value was never properly saved (probably a bug)
        // so explicitly set it to the default

        Config {
            base: Base {
                id: value.base.id.clone(),
                local_validator: value.base.local_validator,
                mixnet_contract_address: value.base.mixnet_contract_address,
                vesting_contract_address: value.base.vesting_contract_address,
                mnemonic: value.base.mnemonic,
            },
            network_monitor: NetworkMonitor {
                enabled: value.network_monitor.enabled,
                storage_paths: NetworkMonitorPaths {
                    credentials_database_path: value
                        .network_monitor
                        .storage_paths
                        .credentials_database_path,
                },
                debug: NetworkMonitorDebug {
                    min_mixnode_reliability: value.network_monitor.debug.min_mixnode_reliability,
                    min_gateway_reliability: value.network_monitor.debug.min_gateway_reliability,
                    disabled_credentials_mode: value
                        .network_monitor
                        .debug
                        .disabled_credentials_mode,
                    run_interval: value.network_monitor.debug.run_interval,
                    gateway_ping_interval: value.network_monitor.debug.gateway_ping_interval,
                    gateway_sending_rate: value.network_monitor.debug.gateway_sending_rate,
                    max_concurrent_gateway_clients: value
                        .network_monitor
                        .debug
                        .max_concurrent_gateway_clients,
                    gateway_response_timeout: value.network_monitor.debug.gateway_response_timeout,
                    gateway_connection_timeout: value
                        .network_monitor
                        .debug
                        .gateway_connection_timeout,
                    packet_delivery_timeout: value.network_monitor.debug.packet_delivery_timeout,
                    test_routes: value.network_monitor.debug.test_routes,
                    minimum_test_routes: value.network_monitor.debug.minimum_test_routes,
                    route_test_packets: value.network_monitor.debug.route_test_packets,
                    per_node_test_packets: value.network_monitor.debug.per_node_test_packets,
                },
            },
            node_status_api: NodeStatusAPI {
                storage_paths: NodeStatusAPIPaths {
                    database_path: value.node_status_api.storage_paths.database_path,
                },
                debug: NodeStatusAPIDebug {
                    caching_interval: value.node_status_api.debug.caching_interval,
                },
            },
            topology_cacher: value.topology_cacher,
            circulating_supply_cacher: value.circulating_supply_cacher,
            rewarding: value.rewarding,
            coconut_signer: CoconutSigner {
                enabled: value.coconut_signer.enabled,
                announce_address: value.base.announce_address,
                storage_paths: CoconutSignerPaths {
                    dkg_persistent_state_path: value
                        .coconut_signer
                        .storage_paths
                        .dkg_persistent_state_path,
                    verification_key_path: value.coconut_signer.storage_paths.verification_key_path,
                    secret_key_path: value.coconut_signer.storage_paths.secret_key_path,
                    decryption_key_path: value.coconut_signer.storage_paths.decryption_key_path,
                    public_key_with_proof_path: value
                        .coconut_signer
                        .storage_paths
                        .public_key_with_proof_path,
                },
                debug: CoconutSignerDebug {
                    dkg_contract_polling_rate: value.coconut_signer.debug.dkg_contract_polling_rate,
                },
            },
            ephemera: Ephemera::new_default(&value.base.id),
        }
    }
}

impl MigrationNymConfig for ConfigV1_1_27 {
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
pub struct BaseV1_1_27 {
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

impl Default for BaseV1_1_27 {
    fn default() -> Self {
        let default_validator: Url = DEFAULT_LOCAL_VALIDATOR
            .parse()
            .expect("default local validator is malformed!");
        let mut default_announce_address = default_validator.clone();
        default_announce_address
            .set_port(Some(DEFAULT_NYM_API_PORT))
            .expect("default local validator is malformed!");

        BaseV1_1_27 {
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
pub struct NetworkMonitorV1_1_27 {
    /// Specifies whether network monitoring service is enabled in this process.
    pub enabled: bool,

    pub storage_paths: NetworkMonitorPaths,

    #[serde(default)]
    pub debug: NetworkMonitorDebug,
}

impl Default for NetworkMonitorV1_1_27 {
    fn default() -> Self {
        NetworkMonitorV1_1_27 {
            enabled: false,
            storage_paths: NetworkMonitorPaths {
                credentials_database_path: Default::default(),
            },
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct NodeStatusAPIV1_1_27 {
    // pub enabled: bool,
    pub storage_paths: NodeStatusAPIPaths,

    #[serde(default)]
    pub debug: NodeStatusAPIDebug,
}

impl Default for NodeStatusAPIV1_1_27 {
    fn default() -> Self {
        NodeStatusAPIV1_1_27 {
            storage_paths: NodeStatusAPIPaths {
                database_path: Default::default(),
            },
            debug: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct CoconutSignerV1_1_27 {
    /// Specifies whether rewarding service is enabled in this process.
    pub enabled: bool,

    pub announce_address: Url,

    pub storage_paths: CoconutSignerPaths,

    #[serde(default)]
    pub debug: CoconutSignerDebug,
}

impl Default for CoconutSignerV1_1_27 {
    fn default() -> Self {
        let announce_address: Url = DEFAULT_LOCAL_VALIDATOR
            .parse()
            .expect("default local validator is malformed!");
        CoconutSignerV1_1_27 {
            enabled: Default::default(),
            announce_address,
            storage_paths: CoconutSignerPaths {
                dkg_persistent_state_path: Default::default(),
                verification_key_path: Default::default(),
                secret_key_path: Default::default(),
                decryption_key_path: Default::default(),
                public_key_with_proof_path: Default::default(),
            },
            debug: Default::default(),
        }
    }
}
