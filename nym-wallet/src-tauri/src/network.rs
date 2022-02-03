// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::Denom;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::str::FromStr;
use strum::EnumIter;

use crate::error::BackendError;
use config::defaults::all::Network as ConfigNetwork;
use config::defaults::{mainnet, qa, sandbox};

#[allow(clippy::upper_case_acronyms)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Copy, Clone, Debug, Deserialize, EnumIter, Eq, Hash, PartialEq, Serialize)]
pub enum Network {
  QA,
  SANDBOX,
  MAINNET,
}

impl Network {
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
    Network::SANDBOX
  }
}

#[allow(clippy::from_over_into)]
impl Into<ConfigNetwork> for Network {
  fn into(self) -> ConfigNetwork {
    match self {
      Network::QA => ConfigNetwork::QA,
      Network::SANDBOX => ConfigNetwork::SANDBOX,
      Network::MAINNET => ConfigNetwork::MAINNET,
    }
  }
}

impl TryFrom<ConfigNetwork> for Network {
  type Error = BackendError;

  fn try_from(value: ConfigNetwork) -> Result<Self, Self::Error> {
    match value {
      ConfigNetwork::QA => Ok(Network::QA),
      ConfigNetwork::SANDBOX => Ok(Network::SANDBOX),
      ConfigNetwork::MAINNET => Ok(Network::MAINNET),
    }
  }
}
