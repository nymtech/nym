// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::var_names::{DEPRECATED_API_VALIDATOR, DEPRECATED_NYMD_VALIDATOR, NYM_API, NYXD};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::{
    env::{var, VarError},
    ffi::OsStr,
    ops::Not,
};
use url::Url;

pub mod mainnet;
pub mod var_names;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
pub struct ChainDetails {
    pub bech32_account_prefix: String,
    pub mix_denom: DenomDetailsOwned,
    pub stake_denom: DenomDetailsOwned,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
pub struct NymContracts {
    pub mixnet_contract_address: Option<String>,
    pub vesting_contract_address: Option<String>,
    pub coconut_bandwidth_contract_address: Option<String>,
    pub group_contract_address: Option<String>,
    pub multisig_contract_address: Option<String>,
    pub coconut_dkg_contract_address: Option<String>,
    pub ephemera_contract_address: Option<String>,
    pub service_provider_directory_contract_address: Option<String>,
    pub name_service_contract_address: Option<String>,
}

// I wanted to use the simpler `NetworkDetails` name, but there's a clash
// with `NetworkDetails` defined in all.rs...
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
pub struct NymNetworkDetails {
    pub network_name: String,
    pub chain_details: ChainDetails,
    pub endpoints: Vec<ValidatorDetails>,
    pub contracts: NymContracts,
    pub explorer_api: Option<String>,
}

// by default we assume the same defaults as mainnet, i.e. same prefixes and denoms
impl Default for NymNetworkDetails {
    fn default() -> Self {
        NymNetworkDetails::new_mainnet()
    }
}

impl NymNetworkDetails {
    pub fn new_empty() -> Self {
        NymNetworkDetails {
            network_name: Default::default(),
            chain_details: ChainDetails {
                bech32_account_prefix: Default::default(),
                mix_denom: DenomDetailsOwned {
                    base: Default::default(),
                    display: Default::default(),
                    display_exponent: Default::default(),
                },
                stake_denom: DenomDetailsOwned {
                    base: Default::default(),
                    display: Default::default(),
                    display_exponent: Default::default(),
                },
            },
            endpoints: Default::default(),
            contracts: Default::default(),
            explorer_api: Default::default(),
        }
    }

    pub fn new_from_env() -> Self {
        fn get_optional_env<K: AsRef<OsStr>>(env: K) -> Option<String> {
            match var(env) {
                Ok(var) => {
                    if var.is_empty() {
                        None
                    } else {
                        Some(var)
                    }
                }
                Err(VarError::NotPresent) => None,
                err => panic!("Unable to set: {:?}", err),
            }
        }

        NymNetworkDetails::new_empty()
            .with_network_name(var(var_names::NETWORK_NAME).expect("network name not set"))
            .with_bech32_account_prefix(
                var(var_names::BECH32_PREFIX).expect("bech32 prefix not set"),
            )
            .with_mix_denom(DenomDetailsOwned {
                base: var(var_names::MIX_DENOM).expect("mix denomination base not set"),
                display: var(var_names::MIX_DENOM_DISPLAY)
                    .expect("mix denomination display not set"),
                display_exponent: var(var_names::DENOMS_EXPONENT)
                    .expect("denomination exponent not set")
                    .parse()
                    .expect("denomination exponent is not u32"),
            })
            .with_stake_denom(DenomDetailsOwned {
                base: var(var_names::STAKE_DENOM).expect("stake denomination base not set"),
                display: var(var_names::STAKE_DENOM_DISPLAY)
                    .expect("stake denomination display not set"),
                display_exponent: var(var_names::DENOMS_EXPONENT)
                    .expect("denomination exponent not set")
                    .parse()
                    .expect("denomination exponent is not u32"),
            })
            .with_additional_validator_endpoint(ValidatorDetails::new(
                var(var_names::NYXD).expect("nyxd validator not set"),
                Some(var(var_names::NYM_API).expect("nym api not set")),
                get_optional_env(var_names::NYXD_WEBSOCKET),
            ))
            .with_mixnet_contract(get_optional_env(var_names::MIXNET_CONTRACT_ADDRESS))
            .with_vesting_contract(get_optional_env(var_names::VESTING_CONTRACT_ADDRESS))
            .with_coconut_bandwidth_contract(get_optional_env(
                var_names::COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
            ))
            .with_group_contract(get_optional_env(var_names::GROUP_CONTRACT_ADDRESS))
            .with_multisig_contract(get_optional_env(var_names::MULTISIG_CONTRACT_ADDRESS))
            .with_coconut_dkg_contract(get_optional_env(var_names::COCONUT_DKG_CONTRACT_ADDRESS))
            .with_ephemera_contract(get_optional_env(var_names::EPHEMERA_CONTRACT_ADDRESS))
            .with_service_provider_directory_contract(get_optional_env(
                var_names::SERVICE_PROVIDER_DIRECTORY_CONTRACT_ADDRESS,
            ))
            .with_name_service_contract(get_optional_env(var_names::NAME_SERVICE_CONTRACT_ADDRESS))
            .with_explorer_api(get_optional_env(var_names::EXPLORER_API))
    }

    pub fn new_mainnet() -> Self {
        fn parse_optional_str(raw: &str) -> Option<String> {
            raw.is_empty().not().then(|| raw.into())
        }

        // Consider caching this process (lazy static)
        NymNetworkDetails {
            network_name: mainnet::NETWORK_NAME.into(),
            chain_details: ChainDetails {
                bech32_account_prefix: mainnet::BECH32_PREFIX.into(),
                mix_denom: mainnet::MIX_DENOM.into(),
                stake_denom: mainnet::STAKE_DENOM.into(),
            },
            endpoints: mainnet::validators(),
            contracts: NymContracts {
                mixnet_contract_address: parse_optional_str(mainnet::MIXNET_CONTRACT_ADDRESS),
                vesting_contract_address: parse_optional_str(mainnet::VESTING_CONTRACT_ADDRESS),
                coconut_bandwidth_contract_address: parse_optional_str(
                    mainnet::COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
                ),
                group_contract_address: parse_optional_str(mainnet::GROUP_CONTRACT_ADDRESS),
                multisig_contract_address: parse_optional_str(mainnet::MULTISIG_CONTRACT_ADDRESS),
                coconut_dkg_contract_address: parse_optional_str(
                    mainnet::COCONUT_DKG_CONTRACT_ADDRESS,
                ),
                ephemera_contract_address: parse_optional_str(mainnet::EPHEMERA_CONTRACT_ADDRESS),
                service_provider_directory_contract_address: None,
                name_service_contract_address: None,
            },
            explorer_api: parse_optional_str(mainnet::EXPLORER_API),
        }
    }

    pub fn default_gas_price_amount(&self) -> f64 {
        GAS_PRICE_AMOUNT
    }

    #[must_use]
    pub fn with_network_name(mut self, network_name: String) -> Self {
        self.network_name = network_name;
        self
    }

    #[must_use]
    pub fn with_chain_details(mut self, chain_details: ChainDetails) -> Self {
        self.chain_details = chain_details;
        self
    }

    #[must_use]
    pub fn with_bech32_account_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.chain_details.bech32_account_prefix = prefix.into();
        self
    }

    #[must_use]
    pub fn with_mix_denom(mut self, mix_denom: DenomDetailsOwned) -> Self {
        self.chain_details.mix_denom = mix_denom;
        self
    }

    #[must_use]
    pub fn with_stake_denom(mut self, stake_denom: DenomDetailsOwned) -> Self {
        self.chain_details.stake_denom = stake_denom;
        self
    }

    #[must_use]
    pub fn with_base_mix_denom<S: Into<String>>(mut self, base_mix_denom: S) -> Self {
        self.chain_details.mix_denom = DenomDetailsOwned::base_only(base_mix_denom.into());
        self
    }

    #[must_use]
    pub fn with_base_stake_denom<S: Into<String>>(mut self, base_stake_denom: S) -> Self {
        self.chain_details.stake_denom = DenomDetailsOwned::base_only(base_stake_denom.into());
        self
    }

    #[must_use]
    pub fn with_additional_validator_endpoint(mut self, endpoint: ValidatorDetails) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    #[must_use]
    pub fn with_validator_endpoint(mut self, endpoint: ValidatorDetails) -> Self {
        self.endpoints = vec![endpoint];
        self
    }

    #[must_use]
    pub fn with_contracts(mut self, contracts: NymContracts) -> Self {
        self.contracts = contracts;
        self
    }

    #[must_use]
    pub fn with_mixnet_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.mixnet_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_vesting_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.vesting_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_coconut_bandwidth_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.coconut_bandwidth_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_group_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.group_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_multisig_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.multisig_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_coconut_dkg_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.coconut_dkg_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_ephemera_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.ephemera_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_service_provider_directory_contract<S: Into<String>>(
        mut self,
        contract: Option<S>,
    ) -> Self {
        self.contracts.service_provider_directory_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_name_service_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.name_service_contract_address = contract.map(Into::into);
        self
    }

    #[must_use]
    pub fn with_explorer_api<S: Into<String>>(mut self, endpoint: Option<S>) -> Self {
        self.explorer_api = endpoint.map(Into::into);
        self
    }
}

#[derive(Debug, Copy, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DenomDetails {
    pub base: &'static str,
    pub display: &'static str,
    // i.e. display_amount * 10^display_exponent = base_amount
    pub display_exponent: u32,
}

impl DenomDetails {
    pub const fn new(base: &'static str, display: &'static str, display_exponent: u32) -> Self {
        DenomDetails {
            base,
            display,
            display_exponent,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Hash, Clone, PartialEq, Eq, JsonSchema)]
pub struct DenomDetailsOwned {
    pub base: String,
    pub display: String,
    // i.e. display_amount * 10^display_exponent = base_amount
    pub display_exponent: u32,
}

impl From<DenomDetails> for DenomDetailsOwned {
    fn from(details: DenomDetails) -> Self {
        DenomDetailsOwned {
            base: details.base.to_owned(),
            display: details.display.to_owned(),
            display_exponent: details.display_exponent,
        }
    }
}

impl DenomDetailsOwned {
    pub fn base_only(base: String) -> Self {
        DenomDetailsOwned {
            base: base.clone(),
            display: base,
            display_exponent: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
pub struct ValidatorDetails {
    // it is assumed those values are always valid since they're being provided in our defaults file
    pub nyxd_url: String,
    //
    pub websocket_url: Option<String>,

    // Right now api_url is optional as we are not running the api reliably on all validators
    // however, later on it should be a mandatory field
    pub api_url: Option<String>,
    // TODO: I'd argue this one should also have a field like `gas_price` since its a validator-specific setting
}

impl ValidatorDetails {
    pub fn new<S: Into<String>>(nyxd_url: S, api_url: Option<S>, websocket_url: Option<S>) -> Self {
        ValidatorDetails {
            nyxd_url: nyxd_url.into(),
            websocket_url: websocket_url.map(Into::into),
            api_url: api_url.map(Into::into),
        }
    }

    pub fn new_nyxd_only<S: Into<String>>(nyxd_url: S) -> Self {
        ValidatorDetails {
            nyxd_url: nyxd_url.into(),
            websocket_url: None,
            api_url: None,
        }
    }

    pub fn nyxd_url(&self) -> Url {
        self.nyxd_url
            .parse()
            .expect("the provided nyxd url is invalid!")
    }

    pub fn api_url(&self) -> Option<Url> {
        self.api_url
            .as_ref()
            .map(|url| url.parse().expect("the provided api url is invalid!"))
    }

    pub fn websocket_url(&self) -> Option<Url> {
        self.websocket_url
            .as_ref()
            .map(|url| url.parse().expect("the provided websocket url is invalid!"))
    }
}

fn fix_deprecated_environmental_variables() {
    // if we're using the outdated environmental variables, set the updated ones to preserve compatibility
    if let Ok(nyxd) = std::env::var(DEPRECATED_NYMD_VALIDATOR) {
        if std::env::var(NYXD).is_err() {
            std::env::set_var(NYXD, nyxd)
        }
    }
    if let Ok(nym_apis) = std::env::var(DEPRECATED_API_VALIDATOR) {
        if std::env::var(NYM_API).is_err() {
            std::env::set_var(NYM_API, nym_apis)
        }
    }
}

// Read the variables from the file and log what the corresponding values in the environment are.
fn print_env_vars_with_keys_in_file<P: AsRef<Path> + Copy>(config_env_file: P) {
    let items = dotenvy::from_path_iter(config_env_file)
        .expect("Invalid path to environment configuration file");
    for item in items {
        let (key, val) = item.expect("Invalid item in environment configuration file");
        log::debug!("{}: {}", key, val);
    }
}

pub fn setup_env<P: AsRef<Path>>(config_env_file: Option<P>) {
    match std::env::var(var_names::CONFIGURED) {
        // if the configuration is not already set in the env vars
        Err(std::env::VarError::NotPresent) => {
            if let Some(config_env_file) = &config_env_file {
                log::debug!(
                    "Loading environment variables from {:?}",
                    config_env_file.as_ref()
                );
                dotenvy::from_path(config_env_file)
                    .expect("Invalid path to environment configuration file");
                fix_deprecated_environmental_variables();
            } else {
                // if nothing is set, the use mainnet defaults
                // if the user has not set `CONFIGURED`, then even if they set any of the env variables,
                // overwrite them
                log::debug!("Loading mainnet defaults");
                crate::mainnet::export_to_env();
            }
        }
        Err(_) => {
            log::debug!("Environment variables already set. Using them");
            crate::mainnet::export_to_env()
        }
        _ => {
            fix_deprecated_environmental_variables();
        }
    }

    // if we haven't explicitly defined any of the constants, fallback to defaults
    crate::mainnet::export_to_env_if_not_set();

    if let Some(config_env_file) = &config_env_file {
        print_env_vars_with_keys_in_file(config_env_file);
    }
}

// Name of the event triggered by the eth contract. If the event name is changed,
// this would also need to be changed; It is currently tested against the json abi
pub const ETH_EVENT_NAME: &str = "BBCredentialPurchased";
pub const ETH_BURN_FUNCTION_NAME: &str = "generateBasicBandwidthCredential";
pub const ETH_ERC20_APPROVE_FUNCTION_NAME: &str = "approve";

// Ethereum constants used for token bridge
/// How much bandwidth (in bytes) one token can buy
pub const BYTES_PER_UTOKEN: u64 = 1024;

/// How much bandwidth (in bytes) one ticket can buy
pub const TICKET_BANDWIDTH_VALUE: u64 = 100 * 1024 * 1024; // 100 MB

/// How much bandwidth (in bytes) one freepass provides
pub const BYTES_PER_FREEPASS: u64 = 10 * 1024 * 1024; // 10 MB

/// Threshold for claiming more bandwidth: 1 MB
pub const REMAINING_BANDWIDTH_THRESHOLD: i64 = 1024 * 1024;
/// How many ERC20 tokens should be burned to buy bandwidth
pub const TOKENS_TO_BURN: u64 = 1;
/// How many ERC20 utokens should be burned to buy bandwidth
pub const UTOKENS_TO_BURN: u64 = TOKENS_TO_BURN * 1000000;
/// Default bandwidth (in bytes) that we try to buy
pub const BANDWIDTH_VALUE: u64 = UTOKENS_TO_BURN * BYTES_PER_UTOKEN;

/// Defaults Cosmos Hub/ATOM path
pub const COSMOS_DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";
// as set by validators in their configs
// (note that the 'amount' postfix is relevant here as the full gas price also includes denom)
pub const GAS_PRICE_AMOUNT: f64 = 0.025;

pub const DEFAULT_MIX_LISTENING_PORT: u16 = 1789;

// 'GATEWAY'
pub const DEFAULT_CLIENT_LISTENING_PORT: u16 = 9000;

// 'MIXNODE'
pub const DEFAULT_VERLOC_LISTENING_PORT: u16 = 1790;
pub const DEFAULT_HTTP_API_LISTENING_PORT: u16 = 8000;

// 'CLIENT'
pub const DEFAULT_WEBSOCKET_LISTENING_PORT: u16 = 1977;

// 'SOCKS5' CLIENT
pub const DEFAULT_SOCKS5_LISTENING_PORT: u16 = 1080;

// NYM-API
pub const DEFAULT_NYM_API_PORT: u16 = 8080;

pub const NYM_API_VERSION: &str = "v1";

// NYM-NODE
pub const DEFAULT_NYM_NODE_HTTP_PORT: u16 = 8080;

// REWARDING

/// We'll be assuming a few more things, profit margin and cost function. Since we don't have reliable package measurement, we'll be using uptime. We'll also set the value of 1 Nym to 1 $, to be able to translate interval costs to Nyms. We'll also assume a cost of 40$ per interval(month), converting that to Nym at our 1$ rate translates to 40_000_000 uNyms
// pub const DEFAULT_OPERATOR_INTERVAL_COST: u64 = 40_000_000; // 40$/(30 days) at 1 Nym == 1$
// pub const DEFAULT_OPERATOR_INTERVAL_COST: u64 = 55_556; // 40$/1hr at 1 Nym == 1$
// pub const DEFAULT_OPERATOR_INTERVAL_COST: u64 = 9259; // 40$/1hr/6 at 1 Nym == 1$

// TODO: is there a way to get this from the chain
pub const TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;

pub const DEFAULT_PROFIT_MARGIN: u8 = 10;

// WIREGUARD
pub const WG_PORT: u16 = 51822;

// The interface used to route traffic
pub const WG_TUN_BASE_NAME: &str = "nymwg";
pub const WG_TUN_DEVICE_ADDRESS: &str = "10.1.0.1";
pub const WG_TUN_DEVICE_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 1, 0, 1));
pub const WG_TUN_DEVICE_NETMASK: &str = "255.255.255.0";
