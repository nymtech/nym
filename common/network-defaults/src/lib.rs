// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use url::Url;

pub mod all;
pub mod eth_contract;
pub mod mainnet;
pub mod qa;
pub mod sandbox;

// The set of defaults that are decided at compile time. Ideally we want to reduce these to a
// minimum.
// Keep DENOM around mostly for use in contracts. (TODO: consider moving it there, or renaming?)
cfg_if::cfg_if! {
    if #[cfg(network = "mainnet")] {
        pub const DEFAULT_NETWORK: all::Network = all::Network::MAINNET;
        pub const DENOM: &str = mainnet::DENOM;
        // pub const STAKE_DENOM: &str = mainnet::STAKE_DENOM;

        pub const ETH_CONTRACT_ADDRESS: [u8; 20] = mainnet::_ETH_CONTRACT_ADDRESS;
        pub const ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] = mainnet::_ETH_ERC20_CONTRACT_ADDRESS;

    } else if #[cfg(network = "qa")] {
        pub const DEFAULT_NETWORK: all::Network = all::Network::QA;
        pub const DENOM: &str = qa::DENOM;
        // pub const STAKE_DENOM: &str = qa::STAKE_DENOM;

        pub const ETH_CONTRACT_ADDRESS: [u8; 20] = qa::_ETH_CONTRACT_ADDRESS;
        pub const ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] = qa::_ETH_ERC20_CONTRACT_ADDRESS;

    } else if #[cfg(network = "sandbox")] {
        pub const DEFAULT_NETWORK: all::Network = all::Network::SANDBOX;
        pub const DENOM: &str = sandbox::DENOM;
        // pub const STAKE_DENOM: &str = sandbox::STAKE_DENOM;

        pub const ETH_CONTRACT_ADDRESS: [u8; 20] = sandbox::_ETH_CONTRACT_ADDRESS;
        pub const ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] = sandbox::_ETH_ERC20_CONTRACT_ADDRESS;
    }
}

// Since these are lazily constructed, we can afford to switch some of them to stronger types in the
// future. If we do this, and also get rid of the references we could potentially unify with
// `NetworkDetails`.
#[derive(Debug)]
pub struct DefaultNetworkDetails {
    bech32_prefix: &'static str,
    mix_denom: DenomDetails,
    stake_denom: DenomDetails,
    mixnet_contract_address: &'static str,
    vesting_contract_address: &'static str,
    bandwidth_claim_contract_address: &'static str,
    rewarding_validator_address: &'static str,
    validators: Vec<ValidatorDetails>,
}

static MAINNET_DEFAULTS: Lazy<DefaultNetworkDetails> = Lazy::new(|| DefaultNetworkDetails {
    bech32_prefix: mainnet::BECH32_PREFIX,
    mix_denom: mainnet::MIX_DENOM,
    stake_denom: mainnet::STAKE_DENOM,
    mixnet_contract_address: mainnet::MIXNET_CONTRACT_ADDRESS,
    vesting_contract_address: mainnet::VESTING_CONTRACT_ADDRESS,
    bandwidth_claim_contract_address: mainnet::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
    rewarding_validator_address: mainnet::REWARDING_VALIDATOR_ADDRESS,
    validators: mainnet::validators(),
});

static SANDBOX_DEFAULTS: Lazy<DefaultNetworkDetails> = Lazy::new(|| DefaultNetworkDetails {
    bech32_prefix: sandbox::BECH32_PREFIX,
    mix_denom: sandbox::MIX_DENOM,
    stake_denom: sandbox::STAKE_DENOM,
    mixnet_contract_address: sandbox::MIXNET_CONTRACT_ADDRESS,
    vesting_contract_address: sandbox::VESTING_CONTRACT_ADDRESS,
    bandwidth_claim_contract_address: sandbox::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
    rewarding_validator_address: sandbox::REWARDING_VALIDATOR_ADDRESS,
    validators: sandbox::validators(),
});

static QA_DEFAULTS: Lazy<DefaultNetworkDetails> = Lazy::new(|| DefaultNetworkDetails {
    bech32_prefix: qa::BECH32_PREFIX,
    mix_denom: qa::MIX_DENOM,
    stake_denom: qa::STAKE_DENOM,
    mixnet_contract_address: qa::MIXNET_CONTRACT_ADDRESS,
    vesting_contract_address: qa::VESTING_CONTRACT_ADDRESS,
    bandwidth_claim_contract_address: qa::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
    rewarding_validator_address: qa::REWARDING_VALIDATOR_ADDRESS,
    validators: qa::validators(),
});

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ValidatorDetails {
    // it is assumed those values are always valid since they're being provided in our defaults file
    pub nymd_url: String,
    // Right now api_url is optional as we are not running the api reliably on all validators
    // however, later on it should be a mandatory field
    pub api_url: Option<String>,
}

impl ValidatorDetails {
    pub fn new(nymd_url: &str, api_url: Option<&str>) -> Self {
        ValidatorDetails {
            nymd_url: nymd_url.to_string(),
            api_url: api_url.map(ToString::to_string),
        }
    }

    pub fn new_with_name(nymd_url: &str, api_url: Option<&str>) -> Self {
        ValidatorDetails {
            nymd_url: nymd_url.to_string(),
            api_url: api_url.map(ToString::to_string),
        }
    }

    pub fn nymd_url(&self) -> Url {
        self.nymd_url
            .parse()
            .expect("the provided nymd url is invalid!")
    }

    pub fn api_url(&self) -> Option<Url> {
        self.api_url
            .as_ref()
            .map(|url| url.parse().expect("the provided api url is invalid!"))
    }
}

pub fn default_nymd_endpoints() -> Vec<Url> {
    DEFAULT_NETWORK
        .validators()
        .map(ValidatorDetails::nymd_url)
        .collect()
}

pub fn default_api_endpoints() -> Vec<Url> {
    DEFAULT_NETWORK
        .validators()
        .filter_map(ValidatorDetails::api_url)
        .collect()
}

// Name of the event triggered by the eth contract. If the event name is changed,
// this would also need to be changed; It is currently tested against the json abi
pub const ETH_EVENT_NAME: &str = "BBCredentialPurchased";
pub const ETH_BURN_FUNCTION_NAME: &str = "generateBasicBandwidthCredential";
pub const ETH_ERC20_APPROVE_FUNCTION_NAME: &str = "approve";

// Ethereum constants used for token bridge
/// How much bandwidth (in bytes) one token can buy
const BYTES_PER_TOKEN: u64 = 1024 * 1024 * 1024;

/// Threshold for claiming more bandwidth: 1 MB
pub const REMAINING_BANDWIDTH_THRESHOLD: i64 = 1024 * 1024;
/// How many ERC20 tokens should be burned to buy bandwidth
pub const TOKENS_TO_BURN: u64 = 1;
/// How many ERC20 utokens should be burned to buy bandwidth
pub const UTOKENS_TO_BURN: u64 = TOKENS_TO_BURN * 1000000;
/// Default bandwidth (in bytes) that we try to buy
pub const BANDWIDTH_VALUE: u64 = TOKENS_TO_BURN * BYTES_PER_TOKEN;

pub const VOUCHER_INFO: &str = "BandwidthVoucher";

pub const ETH_MIN_BLOCK_DEPTH: usize = 7;

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

// VALIDATOR-API
pub const DEFAULT_VALIDATOR_API_PORT: u16 = 8080;

pub const VALIDATOR_API_VERSION: &str = "v1";

// REWARDING

/// We'll be assuming a few more things, profit margin and cost function. Since we don't have relialable package measurement, we'll be using uptime. We'll also set the value of 1 Nym to 1 $, to be able to translate interval costs to Nyms. We'll also assume a cost of 40$ per interval(month), converting that to Nym at our 1$ rate translates to 40_000_000 uNyms
// pub const DEFAULT_OPERATOR_INTERVAL_COST: u64 = 40_000_000; // 40$/(30 days) at 1 Nym == 1$
pub const DEFAULT_OPERATOR_INTERVAL_COST: u64 = 55_556; // 40$/1hr at 1 Nym == 1$

// TODO: is there a way to get this from the chain
pub const TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;

pub const DEFAULT_PROFIT_MARGIN: u8 = 10;
