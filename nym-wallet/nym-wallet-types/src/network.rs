// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_config::defaults::{mainnet, DenomDetails, NymNetworkDetails};
use nym_types::{currency::DecCoin, error::TypesError};
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Not, str::FromStr};
use strum::EnumIter;

mod qa;
mod sandbox;

#[allow(clippy::upper_case_acronyms)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/Network.ts")
)]
#[derive(Copy, Clone, Debug, Deserialize, EnumIter, Eq, Hash, PartialEq, Serialize)]
pub enum Network {
    QA,
    SANDBOX,
    MAINNET,
}

impl Network {
    pub fn as_key(&self) -> String {
        self.to_string().to_lowercase()
    }

    pub fn mix_denom(&self) -> DenomDetails {
        match self {
            Network::QA => qa::MIX_DENOM,
            Network::SANDBOX => sandbox::MIX_DENOM,
            Network::MAINNET => mainnet::MIX_DENOM,
        }
    }

    pub fn base_mix_denom(&self) -> &str {
        match self {
            Network::QA => qa::MIX_DENOM.base,
            Network::SANDBOX => sandbox::MIX_DENOM.base,
            Network::MAINNET => mainnet::MIX_DENOM.base,
        }
    }

    pub fn display_mix_denom(&self) -> &str {
        match self {
            Network::QA => qa::MIX_DENOM.display,
            Network::SANDBOX => sandbox::MIX_DENOM.display,
            Network::MAINNET => mainnet::MIX_DENOM.display,
        }
    }

    pub fn default_zero_mix_display_coin(&self) -> DecCoin {
        DecCoin::zero(self.display_mix_denom())
    }
}

impl Default for Network {
    fn default() -> Self {
        Network::MAINNET
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<Network> for NymNetworkDetails {
    fn from(network: Network) -> Self {
        match network {
            Network::QA => qa::network_details(),
            Network::SANDBOX => sandbox::network_details(),
            Network::MAINNET => NymNetworkDetails::new_mainnet(),
        }
    }
}

impl FromStr for Network {
    type Err = TypesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "qa" => Ok(Network::QA),
            "sandbox" => Ok(Network::SANDBOX),
            "mainnet" => Ok(Network::MAINNET),
            _ => Err(TypesError::UnknownNetwork(s.to_string())),
        }
    }
}

fn parse_optional_str(raw: &str) -> Option<String> {
    raw.is_empty().not().then(|| raw.into())
}
