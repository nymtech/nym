// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network::Network as WalletNetwork;
use config::defaults::{all::SupportedNetworks, ValidatorDetails};
use config::NymConfig;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};
use strum::IntoEnumIterator;
use url::Url;

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
  // Base configuration is not part of the configuration file as it's not intended to be changed.
  #[serde(skip)]
  base: Base,

  // Network level configuration
  network: Network,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Base {
  /// Information on all the networks that the wallet connects to.
  networks: SupportedNetworks,
}

impl Default for Base {
  fn default() -> Self {
    let networks = WalletNetwork::iter()
      .map(|network| network.into())
      .collect();
    Base {
      networks: SupportedNetworks::new(networks),
    }
  }
}

impl NymConfig for Config {
  fn template() -> &'static str {
    // For now we're not using a template
    unimplemented!();
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

  fn save_to_file(&self, custom_location: Option<PathBuf>) -> io::Result<()> {
    let config_toml = toml::to_string_pretty(&self)
      .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))?;

    // Make sure the whole directory structure actually exists
    match custom_location.clone() {
      Some(loc) => {
        if let Some(parent_dir) = loc.parent() {
          fs::create_dir_all(parent_dir)
        } else {
          Ok(())
        }
      }
      None => fs::create_dir_all(self.config_directory()),
    }?;

    fs::write(
      custom_location.unwrap_or_else(|| self.config_directory().join(Self::config_file_name())),
      config_toml,
    )
  }
}

impl Config {
  pub fn get_nymd_validator_url(&self, network: WalletNetwork) -> Url {
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

  pub fn get_validator_api_url(&self, network: WalletNetwork) -> Url {
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

  pub fn get_mixnet_contract_address(&self, network: WalletNetwork) -> Option<cosmrs::AccountId> {
    self
      .base
      .networks
      .mixnet_contract_address(network.into())
      .expect("No mixnet contract address found in config")
      .parse()
      .ok()
  }

  pub fn get_vesting_contract_address(&self, network: WalletNetwork) -> Option<cosmrs::AccountId> {
    self
      .base
      .networks
      .vesting_contract_address(network.into())
      .expect("No vesting contract address found in config")
      .parse()
      .ok()
  }

  pub fn get_bandwidth_claim_contract_address(
    &self,
    network: WalletNetwork,
  ) -> Option<cosmrs::AccountId> {
    self
      .base
      .networks
      .bandwidth_claim_contract_address(network.into())
      .expect("No bandwidth claim contract address found in config")
      .parse()
      .ok()
  }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Network {
  // User supplied additional validator urls in addition to the hardcoded ones.
  // NOTE: these are separate fields, rather than a map, to force the serialization order.
  mainnet: Option<Vec<ValidatorDetails>>,
  sandbox: Option<Vec<ValidatorDetails>>,
  qa: Option<Vec<ValidatorDetails>>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use config::defaults::all::Network as NetworkConfig;

  fn test_config() -> Config {
    Config {
      base: Base::default(),
      network: Network {
        mainnet: Some(vec![
          // Add the default one, although the hardcoded default isn't intended to be included in
          // the config file.
          NetworkConfig::MAINNET.validators().next().unwrap().clone(),
          // An additional one
          ValidatorDetails {
            nymd_url: "https://42".to_string(),
            api_url: None,
          },
        ]),
        sandbox: Some(NetworkConfig::SANDBOX.validators().cloned().collect()),
        qa: None,
      },
    }
  }

  #[test]
  fn serialize_to_toml() {
    assert_eq!(
      toml::to_string_pretty(&test_config()).unwrap(),
      r#"[[network.mainnet]]
nymd_url = 'https://rpc.nyx.nodes.guru/'
api_url = 'https://api.nyx.nodes.guru/'

[[network.mainnet]]
nymd_url = 'https://42'

[[network.sandbox]]
nymd_url = 'https://sandbox-validator.nymtech.net'
api_url = 'https://sandbox-validator.nymtech.net/api'
"#
    );
  }
  #[test]
  fn serialize_and_deserialize_to_toml() {
    let config = test_config();
    let config_str = toml::to_string_pretty(&config).unwrap();
    let config_from_toml = toml::from_str(&config_str).unwrap();
    assert_eq!(config, config_from_toml);
  }
}
