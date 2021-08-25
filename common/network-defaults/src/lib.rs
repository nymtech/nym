// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use time::OffsetDateTime;
use url::Url;

pub struct ValidatorDetails<'a> {
    // it is assumed those values are always valid since they're being provided in our defaults file
    pub nymd_url: &'a str,
    // Right now api_url is optional as we are not running the api reliably on all validators
    // however, later on it should be a mandatory field
    pub api_url: Option<&'a str>,
}

impl<'a> ValidatorDetails<'a> {
    pub const fn new(nymd_url: &'a str, api_url: Option<&'a str>) -> Self {
        ValidatorDetails { nymd_url, api_url }
    }

    pub fn nymd_url(&self) -> Url {
        self.nymd_url
            .parse()
            .expect("the provided nymd url is invalid!")
    }

    pub fn api_url(&self) -> Option<Url> {
        self.api_url
            .map(|url| url.parse().expect("the provided api url is invalid!"))
    }
}

pub const DEFAULT_VALIDATORS: &[ValidatorDetails] = &[
    ValidatorDetails::new(
        "https://testnet-milhon-validator1.nymtech.net",
        Some("https://testnet-milhon-validator1.nymtech.net/api"),
    ),
    ValidatorDetails::new("https://testnet-milhon-validator2.nymtech.net", None),
];

pub fn default_nymd_endpoints() -> Vec<Url> {
    DEFAULT_VALIDATORS
        .iter()
        .map(|validator| validator.nymd_url())
        .collect()
}

pub fn default_api_endpoints() -> Vec<Url> {
    DEFAULT_VALIDATORS
        .iter()
        .filter_map(|validator| validator.api_url())
        .collect()
}

pub const DEFAULT_MIXNET_CONTRACT_ADDRESS: &str = "punk10pyejy66429refv3g35g2t7am0was7yalwrzen";
pub const NETWORK_MONITOR_ADDRESS: &str = "punk1v9qauwdq5terag6uvfsdytcs2d0sdmfdy7hgk3";

/// Defaults Cosmos Hub/ATOM path
pub const COSMOS_DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";
pub const BECH32_PREFIX: &str = "punk";
pub const DENOM: &str = "upunk";
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
pub const DEFAULT_FIRST_EPOCH: OffsetDateTime = time::macros::datetime!(2021-08-23 12:00 UTC);
pub const DEFAULT_EPOCH_LENGTH: Duration = Duration::from_secs(24 * 60 * 60); // 24h
