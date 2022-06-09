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

    pub fn denom(&self) -> Denom {
        match self {
            // network defaults should be correctly formatted
            Network::QA => Denom::from_str(qa::DENOM).unwrap(),
            Network::SANDBOX => Denom::from_str(sandbox::DENOM).unwrap(),
            Network::MAINNET => Denom::from_str(mainnet::DENOM).unwrap(),
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
