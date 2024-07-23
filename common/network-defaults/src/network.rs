// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{mainnet, GAS_PRICE_AMOUNT};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Not;
use url::Url;

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
    pub ecash_contract_address: Option<String>,
    pub group_contract_address: Option<String>,
    pub multisig_contract_address: Option<String>,
    pub coconut_dkg_contract_address: Option<String>,
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

    #[cfg(feature = "env")]
    pub fn new_from_env() -> Self {
        use crate::var_names;
        use std::env::{var, VarError};
        use std::ffi::OsStr;

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
            .with_ecash_contract(get_optional_env(var_names::ECASH_CONTRACT_ADDRESS))
            .with_group_contract(get_optional_env(var_names::GROUP_CONTRACT_ADDRESS))
            .with_multisig_contract(get_optional_env(var_names::MULTISIG_CONTRACT_ADDRESS))
            .with_coconut_dkg_contract(get_optional_env(var_names::COCONUT_DKG_CONTRACT_ADDRESS))
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
                ecash_contract_address: parse_optional_str(mainnet::ECASH_CONTRACT_ADDRESS),
                group_contract_address: parse_optional_str(mainnet::GROUP_CONTRACT_ADDRESS),
                multisig_contract_address: parse_optional_str(mainnet::MULTISIG_CONTRACT_ADDRESS),
                coconut_dkg_contract_address: parse_optional_str(
                    mainnet::COCONUT_DKG_CONTRACT_ADDRESS,
                ),
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
    pub fn with_ecash_contract<S: Into<String>>(mut self, contract: Option<S>) -> Self {
        self.contracts.ecash_contract_address = contract.map(Into::into);
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
