// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{GAS_PRICE_AMOUNT, mainnet};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

pub mod v1;
pub mod v2;
pub type NymNetworkDetails = v2::NymNetworkDetails;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ChainDetails {
    pub bech32_account_prefix: String,
    pub mix_denom: DenomDetailsOwned,
    pub stake_denom: DenomDetailsOwned,
}

impl ChainDetails {
    pub fn mainnet() -> Self {
        ChainDetails {
            bech32_account_prefix: mainnet::BECH32_PREFIX.into(),
            mix_denom: mainnet::MIX_DENOM.into(),
            stake_denom: mainnet::STAKE_DENOM.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct NymContracts {
    pub mixnet_contract_address: Option<String>,
    pub vesting_contract_address: Option<String>,
    #[serde(default)]
    pub performance_contract_address: Option<String>,
    #[serde(default)]
    pub network_monitors_contract_address: Option<String>,
    #[serde(default)]
    pub node_families_contract_address: Option<String>,
    pub ecash_contract_address: Option<String>,
    pub group_contract_address: Option<String>,
    pub multisig_contract_address: Option<String>,
    pub coconut_dkg_contract_address: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, JsonSchema)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct ApiUrl {
    /// Expects a string formatted Url
    ///
    /// see https://docs.rs/url/latest/url/struct.Url.html
    pub url: String,
    /// Optional alternative equivalent hostnames. Each entry must parse as valid Host
    ///
    /// see https://docs.rs/url/latest/url/enum.Host.html
    pub front_hosts: Option<Vec<String>>,
}

impl From<Url> for ApiUrl {
    fn from(value: Url) -> Self {
        ApiUrl {
            url: value.to_string(),
            front_hosts: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct ApiUrlConst<'a> {
    pub url: &'a str,
    pub front_hosts: Option<&'a [&'a str]>,
}

impl From<ApiUrlConst<'_>> for ApiUrl {
    fn from(value: ApiUrlConst) -> Self {
        ApiUrl {
            url: value.url.to_string(),
            front_hosts: value
                .front_hosts
                .map(|slice| slice.iter().map(|s| s.to_string()).collect()),
        }
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
