// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;
use url::Url;

pub mod all;
pub mod eth_contract;
mod milhon;
mod qa;
mod sandbox;

cfg_if::cfg_if! {
    if #[cfg(network = "milhon")] {
        pub const BECH32_PREFIX: &str = milhon::BECH32_PREFIX;
        pub const DENOM: &str = milhon::DENOM;

        pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = milhon::MIXNET_CONTRACT_ADDRESS;
        pub const DEFAULT_VESTING_CONTRACT_ADDRESS: &str = milhon::VESTING_CONTRACT_ADDRESS;
        pub const DEFAULT_BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str = milhon::BANDWIDTH_CLAIM_CONTRACT_ADDRESS;
        pub const DEFAULT_REWARDING_VALIDATOR_ADDRESS: &str = milhon::REWARDING_VALIDATOR_ADDRESS;

        pub fn default_validators() -> Vec<ValidatorDetails> {
            milhon::validators()
        }

        pub fn default_network() -> all::Network {
            all::Network::MILHON
        }
    } else if #[cfg(network = "qa")] {
        pub const BECH32_PREFIX: &str = qa::BECH32_PREFIX;
        pub const DENOM: &str = qa::DENOM;

        pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = qa::MIXNET_CONTRACT_ADDRESS;
        pub const DEFAULT_VESTING_CONTRACT_ADDRESS: &str = qa::VESTING_CONTRACT_ADDRESS;
        pub const DEFAULT_BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str = qa::BANDWIDTH_CLAIM_CONTRACT_ADDRESS;
        pub const DEFAULT_REWARDING_VALIDATOR: &str = qa::REWARDING_VALIDATOR_ADDRESS;

        pub fn default_validators() -> Vec<ValidatorDetails> {
            qa::validators()
        }

        pub fn default_network() -> all::Network {
            all::Network::QA
        }
    } else if #[cfg(network = "sandbox")] {
        pub const BECH32_PREFIX: &str = sandbox::BECH32_PREFIX;
        pub const DENOM: &str = sandbox::DENOM;

        pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = sandbox::MIXNET_CONTRACT_ADDRESS;
        pub const DEFAULT_VESTING_CONTRACT_ADDRESS: &str = sandbox::VESTING_CONTRACT_ADDRESS;
        pub const DEFAULT_BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str = sandbox::BANDWIDTH_CLAIM_CONTRACT_ADDRESS;
        pub const DEFAULT_REWARDING_VALIDATOR: &str = sandbox::REWARDING_VALIDATOR_ADDRESS;

        pub fn default_validators() -> Vec<ValidatorDetails> {
            sandbox::validators()
        }

        pub fn default_network() -> all::Network {
            all::Network::SANDBOX
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidatorDetails {
    // it is assumed those values are always valid since they're being provided in our defaults file
    pub nymd_url: String,
    // Right now api_url is optional as we are not running the api reliably on all validators
    // however, later on it should be a mandatory field
    pub api_url: Option<String>,
}

impl ValidatorDetails {
    pub fn new(nymd_url: &str, api_url: Option<&str>) -> Self {
        let api_url = api_url.map(|api_url_str| api_url_str.to_string());
        ValidatorDetails {
            nymd_url: nymd_url.to_string(),
            api_url,
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
    default_validators()
        .iter()
        .map(|validator| validator.nymd_url())
        .collect()
}

pub fn default_api_endpoints() -> Vec<Url> {
    default_validators()
        .iter()
        .filter_map(|validator| validator.api_url())
        .collect()
}

pub const ETH_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("9fEE3e28c17dbB87310A51F13C4fbf4331A6f102");
// Name of the event triggered by the eth contract. If the event name is changed,
// this would also need to be changed; It is currently tested against the json abi
pub const ETH_EVENT_NAME: &str = "Burned";
pub const ETH_BURN_FUNCTION_NAME: &str = "burnTokenForAccessCode";

// Ethereum constants used for token bridge
/// How much bandwidth (in bytes) one token can buy
const BYTES_PER_TOKEN: u64 = 1024 * 1024 * 1024;
/// How many ERC20 tokens should be burned to buy bandwidth
pub const TOKENS_TO_BURN: u64 = 10;
/// Default bandwidth (in bytes) that we try to buy
pub const BANDWIDTH_VALUE: u64 = TOKENS_TO_BURN * BYTES_PER_TOKEN;

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
pub const DEFAULT_FIRST_EPOCH_START: OffsetDateTime = time::macros::datetime!(2021-08-23 12:00 UTC);
pub const DEFAULT_EPOCH_LENGTH: Duration = Duration::from_secs(24 * 60 * 60 * 30); // 30 days, i.e. 720h
/// We'll be assuming a few more things, profit margin and cost function. Since we don't have relialable package measurement, we'll be using uptime. We'll also set the value of 1 Nym to 1 $, to be able to translate epoch costs to Nyms. We'll also assume a cost of 40$ per epoch(month), converting that to Nym at our 1$ rate translates to 40_000_000 uNyms
pub const DEFAULT_OPERATOR_EPOCH_COST: u64 = 40_000_000; // 40$/(30 days) at 1 Nym == 1$

// TODO: is there a way to get this from the chain
pub const TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;

pub const DEFAULT_PROFIT_MARGIN: u8 = 10;
