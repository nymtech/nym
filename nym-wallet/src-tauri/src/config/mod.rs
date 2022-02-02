// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network::Network;
use config::defaults::all::SupportedNetworks;
use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum::IntoEnumIterator;
use url::Url;

mod template;

use template::config_template;

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
  base: Base,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Base {
  /// Information on all the networks that the wallet connects to.
  networks: SupportedNetworks,
}

impl Default for Base {
  fn default() -> Self {
    let networks = Network::iter().map(|network| network.into()).collect();
    Base {
      networks: SupportedNetworks::new(networks),
    }
  }
}

impl NymConfig for Config {
  fn template() -> &'static str {
    config_template()
  }

  fn default_root_directory() -> PathBuf {
    dirs::home_dir()
      .expect("Failed to evaluate $HOME value")
      .join(".nym")
      .join("wallet")
  }

  fn root_directory(&self) -> PathBuf {
    Self::default_root_directory()
  }

  fn config_directory(&self) -> PathBuf {
    self.root_directory().join("config")
  }

  fn data_directory(&self) -> PathBuf {
    self.root_directory().join("data")
  }
}

impl Config {
  pub fn get_nymd_validator_url(&self, network: Network) -> Url {
    // TODO make this a random choice
    if let Some(Some(validator_details)) = self
      .base
      .networks
      .validators(network.into())
      .map(|validators| validators.first())
    {
      validator_details.nymd_url()
    } else {
      panic!("No validators found in config")
    }
  }

  pub fn get_validator_api_url(&self, network: Network) -> Url {
    // TODO make this a random choice
    if let Some(Some(validator_details)) = self
      .base
      .networks
      .validators(network.into())
      .map(|validators| validators.first())
    {
      validator_details.api_url().expect("no api url provided")
    } else {
      panic!("No validators found in config")
    }
  }

  pub fn get_mixnet_contract_address(&self, network: Network) -> Option<cosmrs::AccountId> {
    self
      .base
      .networks
      .mixnet_contract_address(network.into())
      .expect("No mixnet contract address found in config")
      .parse()
      .ok()
  }

  pub fn get_vesting_contract_address(&self, network: Network) -> Option<cosmrs::AccountId> {
    self
      .base
      .networks
      .vesting_contract_address(network.into())
      .expect("No vesting contract address found in config")
      .parse()
      .ok()
  }
}
