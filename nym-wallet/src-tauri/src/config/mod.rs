// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::{default_validators, ValidatorDetails, DEFAULT_MIXNET_CONTRACT_ADDRESS};
use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
  validators: Vec<ValidatorDetails>,

  /// Address of the validator contract managing the network
  mixnet_contract_address: String,

  /// Mnemonic (currently of the network monitor) used for rewarding
  mnemonic: String,
}

impl Default for Base {
  fn default() -> Self {
    Base {
      validators: default_validators(),
      mixnet_contract_address: DEFAULT_MIXNET_CONTRACT_ADDRESS.to_string(),
      mnemonic: String::default(),
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
  pub fn get_nymd_validator_url(&self) -> Url {
    // TODO make this a random choice
    if let Some(validator_details) = self.base.validators.first() {
      validator_details.nymd_url()
    } else {
      panic!("No validators found in config")
    }
  }

  pub fn get_validator_api_url(&self) -> Url {
    // TODO make this a random choice
    if let Some(validator_details) = self.base.validators.first() {
      validator_details.api_url().expect("no api url provided")
    } else {
      panic!("No validators found in config")
    }
  }

  pub fn get_mixnet_contract_address(&self) -> cosmrs::AccountId {
    self
      .base
      .mixnet_contract_address
      .parse()
      .expect("stored mixnet contract address is not a valid account address")
  }

  pub fn get_vesting_contract_address(&self) -> Option<cosmrs::AccountId> {
    None
  }
}
