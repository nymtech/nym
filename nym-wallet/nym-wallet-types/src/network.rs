// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::Denom;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use strum::EnumIter;

use config::defaults::all::Network as ConfigNetwork;
use config::defaults::{mainnet, qa, sandbox};

#[allow(clippy::upper_case_acronyms)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/Network.ts")
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

    // this should be returning just a `&str`, but don't want to cause too many conflicts just yet...
    pub fn base_mix_denom(&self) -> Denom {
        match self {
            // network defaults should be correctly formatted
            Network::QA => Denom::from_str(qa::MIX_DENOM.base).unwrap(),
            Network::SANDBOX => Denom::from_str(sandbox::MIX_DENOM.base).unwrap(),
            Network::MAINNET => Denom::from_str(mainnet::MIX_DENOM.base).unwrap(),
        }
    }

    pub fn display_mix_denom(&self) -> &str {
        match self {
            Network::QA => qa::MIX_DENOM.display,
            Network::SANDBOX => sandbox::MIX_DENOM.display,
            Network::MAINNET => mainnet::MIX_DENOM.display,
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Network::MAINNET
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ConfigNetwork> for Network {
    fn from(network: ConfigNetwork) -> Self {
        match network {
            ConfigNetwork::QA => Network::QA,
            ConfigNetwork::SANDBOX => Network::SANDBOX,
            ConfigNetwork::MAINNET => Network::MAINNET,
            ConfigNetwork::CUSTOM { .. } => panic!("custom network is not supported"),
        }
    }
}

impl From<Network> for ConfigNetwork {
    fn from(network: Network) -> Self {
        match network {
            Network::QA => ConfigNetwork::QA,
            Network::SANDBOX => ConfigNetwork::SANDBOX,
            Network::MAINNET => ConfigNetwork::MAINNET,
        }
    }
}
