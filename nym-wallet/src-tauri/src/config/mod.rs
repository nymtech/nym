// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_config;
use crate::platform_constants::{CONFIG_DIR_NAME, CONFIG_FILENAME};
use crate::{error::BackendError, network::Network as WalletNetwork};
use config::defaults::all::Network;
use config::defaults::{all::SupportedNetworks, ValidatorDetails};
use core::fmt;
use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::{fs, io, path::PathBuf};
use strum::IntoEnumIterator;
use url::Url;

pub const REMOTE_SOURCE_OF_VALIDATOR_URLS: &str =
  "https://nymtech.net/.wellknown/wallet/validators.json";

const CURRENT_GLOBAL_CONFIG_VERSION: u32 = 1;
const CURRENT_NETWORK_CONFIG_VERSION: u32 = 1;
pub(crate) const CUSTOM_SIMULATED_GAS_MULTIPLIER: f32 = 1.4;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
  // Base configuration is not part of the configuration file as it's not intended to be changed.
  base: Base,

  // Global configuration file
  global: Option<GlobalConfig>,

  // One configuration file per network
  networks: HashMap<String, NetworkConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
struct Base {
  /// Information on all the networks that the wallet connects to.
  networks: SupportedNetworks,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct GlobalConfig {
  version: Option<u32>,
  // TODO: there are no global settings (yet)
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct NetworkConfig {
  version: Option<u32>,

  // User selected urls
  selected_nymd_url: Option<Url>,
  selected_api_url: Option<Url>,

  // Additional user provided validators.
  // It is an option for the purpuse of file serialization.
  validator_urls: Option<Vec<ValidatorUrl>>,
}

impl Default for Base {
  fn default() -> Self {
    let networks = WalletNetwork::iter().map(Into::into).collect();
    Base {
      networks: SupportedNetworks::new(networks),
    }
  }
}

impl Default for GlobalConfig {
  fn default() -> Self {
    Self {
      version: Some(CURRENT_GLOBAL_CONFIG_VERSION),
    }
  }
}

impl Default for NetworkConfig {
  fn default() -> Self {
    Self {
      version: Some(CURRENT_NETWORK_CONFIG_VERSION),
      selected_nymd_url: None,
      selected_api_url: None,
      validator_urls: None,
    }
  }
}

impl NetworkConfig {
  fn validators(&self) -> impl Iterator<Item = &ValidatorUrl> {
    self.validator_urls.iter().flat_map(|v| v.iter())
  }
}

impl Config {
  fn root_directory() -> PathBuf {
    tauri::api::path::config_dir().expect("Failed to get config directory")
  }

  fn config_directory() -> PathBuf {
    Self::root_directory().join(CONFIG_DIR_NAME)
  }

  fn config_file_path(network: Option<WalletNetwork>) -> PathBuf {
    if let Some(network) = network {
      let network_filename = format!("{}.toml", network.as_key());
      Self::config_directory().join(network_filename)
    } else {
      Self::config_directory().join(CONFIG_FILENAME)
    }
  }

  pub fn save_to_files(&self) -> io::Result<()> {
    log::trace!("Config::save_to_file");

    // Make sure the whole directory structure actually exists
    fs::create_dir_all(Self::config_directory())?;

    // Global config
    if let Some(global) = &self.global {
      let location = Self::config_file_path(None);

      match toml::to_string_pretty(&global)
        .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
        .map(|toml| fs::write(location.clone(), toml))
      {
        Ok(_) => log::debug!("Writing to: {:#?}", location),
        Err(err) => log::warn!("Failed to write to {:#?}: {err}", location),
      }
    }

    // One file per network
    for (network, config) in &self.networks {
      let network = match Network::from_str(network).map(Into::into) {
        Ok(network) => network,
        Err(err) => {
          log::warn!("Unexpected name for network configuration, not saving: {err}");
          break;
        }
      };

      let location = Self::config_file_path(Some(network));
      match toml::to_string_pretty(config)
        .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
        .map(|toml| fs::write(location.clone(), toml))
      {
        Ok(_) => log::debug!("Writing to: {:#?}", location),
        Err(err) => log::warn!("Failed to write to {:#?}: {err}", location),
      }
    }
    Ok(())
  }

  pub fn load_from_files() -> Self {
    // Global
    let global = {
      let file = Self::config_file_path(None);
      match load_from_file::<GlobalConfig>(file.clone()) {
        Ok(global) => {
          log::debug!("Loaded from file {:#?}", file);
          Some(global)
        }
        Err(err) => {
          log::trace!("Not loading {:#?}: {}", file, err);
          None
        }
      }
    };

    // One file per network
    let mut networks = HashMap::new();
    for network in WalletNetwork::iter() {
      let file = Self::config_file_path(Some(network));
      match load_from_file::<NetworkConfig>(file.clone()) {
        Ok(config) => {
          log::trace!("Loaded from file {:#?}", file);
          networks.insert(network.as_key(), config);
        }
        Err(err) => log::trace!("Not loading {:#?}: {}", file, err),
      };
    }

    Self {
      base: Base::default(),
      global,
      networks,
    }
  }

  pub fn get_base_validators(
    &self,
    network: WalletNetwork,
  ) -> impl Iterator<Item = ValidatorUrl> + '_ {
    self.base.networks.validators(network.into()).map(|v| {
      v.clone()
        .try_into()
        .expect("The hardcoded validators are assumed to be valid urls")
    })
  }

  pub fn get_configured_validators(
    &self,
    network: WalletNetwork,
  ) -> impl Iterator<Item = ValidatorUrl> + '_ {
    self
      .networks
      .get(&network.as_key())
      .into_iter()
      .flat_map(|c| c.validators().cloned())
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

  pub fn select_validator_nymd_url(&mut self, nymd_url: Url, network: WalletNetwork) {
    if let Some(net) = self.networks.get_mut(&network.as_key()) {
      net.selected_nymd_url = Some(nymd_url);
    } else {
      self.networks.insert(
        network.as_key(),
        NetworkConfig {
          selected_nymd_url: Some(nymd_url),
          ..NetworkConfig::default()
        },
      );
    }
  }

  pub fn select_validator_api_url(&mut self, api_url: Url, network: WalletNetwork) {
    if let Some(net) = self.networks.get_mut(&network.as_key()) {
      net.selected_api_url = Some(api_url);
    } else {
      self.networks.insert(
        network.as_key(),
        NetworkConfig {
          selected_nymd_url: Some(api_url),
          ..NetworkConfig::default()
        },
      );
    }
  }

  pub fn get_selected_validator_nymd_url(&self, network: &WalletNetwork) -> Option<Url> {
    self
      .networks
      .get(&network.as_key())
      .and_then(|config| config.selected_nymd_url.clone())
  }

  pub fn get_selected_validator_api_url(&self, network: &WalletNetwork) -> Option<Url> {
    self
      .networks
      .get(&network.as_key())
      .and_then(|config| config.selected_api_url.clone())
  }

  pub fn add_validator_url(&mut self, url: ValidatorUrl, network: WalletNetwork) {
    if let Some(network_config) = self.networks.get_mut(&network.as_key()) {
      if let Some(ref mut urls) = network_config.validator_urls {
        urls.push(url);
      } else {
        network_config.validator_urls = Some(vec![url]);
      }
    } else {
      self.networks.insert(
        network.as_key(),
        NetworkConfig {
          validator_urls: Some(vec![url]),
          ..NetworkConfig::default()
        },
      );
    }
  }

  pub fn remove_validator_url(&mut self, url: ValidatorUrl, network: WalletNetwork) {
    if let Some(network_config) = self.networks.get_mut(&network.as_key()) {
      if let Some(ref mut urls) = network_config.validator_urls {
        // Removes duplicates too if there are any
        urls.retain(|existing_url| existing_url != &url);
      }
    }
  }
}

fn load_from_file<T>(file: PathBuf) -> Result<T, io::Error>
where
  T: DeserializeOwned,
{
  fs::read_to_string(file).and_then(|contents| {
    toml::from_str::<T>(&contents)
      .map_err(|toml_err| io::Error::new(io::ErrorKind::Other, toml_err))
  })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ValidatorUrl {
  pub nymd_url: Url,
  pub api_url: Option<Url>,
}

impl TryFrom<ValidatorDetails> for ValidatorUrl {
  type Error = BackendError;

  fn try_from(validator: ValidatorDetails) -> Result<Self, Self::Error> {
    Ok(ValidatorUrl {
      nymd_url: validator.nymd_url.parse()?,
      api_url: match &validator.api_url {
        Some(url) => Some(url.parse()?),
        None => None,
      },
    })
  }
}

impl TryFrom<network_config::Validator> for ValidatorUrl {
  type Error = BackendError;

  fn try_from(validator: network_config::Validator) -> Result<Self, Self::Error> {
    Ok(ValidatorUrl {
      nymd_url: validator.nymd_url.parse()?,
      api_url: match &validator.api_url {
        Some(url) => Some(url.parse()?),
        None => None,
      },
    })
  }
}

impl fmt::Display for ValidatorUrl {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s1 = format!("nymd_url: {}", self.nymd_url);
    let s2 = self
      .api_url
      .as_ref()
      .map(|url| format!(", api_url: {}", url));
    write!(f, "    {}{},", s1, s2.unwrap_or_default())
  }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptionalValidators {
  // User supplied additional validator urls in addition to the hardcoded ones.
  // These are separate fields, rather than a map, to force the serialization order.
  mainnet: Option<Vec<ValidatorUrl>>,
  sandbox: Option<Vec<ValidatorUrl>>,
  qa: Option<Vec<ValidatorUrl>>,
}

impl OptionalValidators {
  pub fn validators(&self, network: WalletNetwork) -> impl Iterator<Item = &ValidatorUrl> {
    match network {
      WalletNetwork::MAINNET => self.mainnet.as_ref(),
      WalletNetwork::SANDBOX => self.sandbox.as_ref(),
      WalletNetwork::QA => self.qa.as_ref(),
    }
    .into_iter()
    .flatten()
  }
}

impl fmt::Display for OptionalValidators {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s1 = self
      .mainnet
      .as_ref()
      .map(|validators| format!("mainnet: [\n{}\n]", validators.iter().format("\n")))
      .unwrap_or_default();
    let s2 = self
      .sandbox
      .as_ref()
      .map(|validators| format!(",\nsandbox: [\n{}\n]", validators.iter().format("\n")))
      .unwrap_or_default();
    let s3 = self
      .qa
      .as_ref()
      .map(|validators| format!(",\nqa: [\n{}\n]", validators.iter().format("\n")))
      .unwrap_or_default();
    write!(f, "{}{}{}", s1, s2, s3)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn test_config() -> Config {
    let netconfig = NetworkConfig {
      selected_nymd_url: None,
      selected_api_url: Some("https://my_api_url.com".parse().unwrap()),

      validator_urls: Some(vec![
        ValidatorUrl {
          nymd_url: "https://foo".parse().unwrap(),
          api_url: None,
        },
        ValidatorUrl {
          nymd_url: "https://bar".parse().unwrap(),
          api_url: Some("https://bar/api".parse().unwrap()),
        },
        ValidatorUrl {
          nymd_url: "https://baz".parse().unwrap(),
          api_url: Some("https://baz/api".parse().unwrap()),
        },
      ]),
      ..NetworkConfig::default()
    };

    Config {
      base: Base::default(),
      global: Some(GlobalConfig::default()),
      networks: [(WalletNetwork::MAINNET.as_key(), netconfig)]
        .into_iter()
        .collect(),
    }
  }

  #[test]
  fn serialize_to_toml() {
    let config = test_config();
    let netconfig = &config.networks[&WalletNetwork::MAINNET.as_key()];
    assert_eq!(
      toml::to_string_pretty(netconfig).unwrap(),
      r#"version = 1
selected_api_url = 'https://my_api_url.com/'

[[validator_urls]]
nymd_url = 'https://foo/'

[[validator_urls]]
nymd_url = 'https://bar/'
api_url = 'https://bar/api'

[[validator_urls]]
nymd_url = 'https://baz/'
api_url = 'https://baz/api'
"#
    );
  }
  #[test]
  fn serialize_and_deserialize_to_toml() {
    let config = test_config();
    let netconfig = &config.networks[&WalletNetwork::MAINNET.as_key()];
    let config_str = toml::to_string_pretty(netconfig).unwrap();
    let config_from_toml: NetworkConfig = toml::from_str(&config_str).unwrap();
    assert_eq!(netconfig, &config_from_toml);
  }

  #[test]
  fn get_urls_parsed_from_config() {
    let config = test_config();

    let nymd_url = config
      .get_configured_validators(WalletNetwork::MAINNET)
      .next()
      .map(|v| v.nymd_url)
      .unwrap();
    assert_eq!(nymd_url.as_ref(), "https://foo/");

    // The first entry is missing an API URL
    let api_url = config
      .get_configured_validators(WalletNetwork::MAINNET)
      .next()
      .and_then(|v| v.api_url);
    assert_eq!(api_url, None);
  }

  #[test]
  fn get_urls_from_defaults() {
    let config = Config::default();

    let nymd_url = config
      .get_base_validators(WalletNetwork::MAINNET)
      .next()
      .map(|v| v.nymd_url)
      .unwrap();
    assert_eq!(nymd_url.as_ref(), "https://rpc.nyx.nodes.guru/");

    let api_url = config
      .get_base_validators(WalletNetwork::MAINNET)
      .next()
      .and_then(|v| v.api_url)
      .unwrap();
    assert_eq!(api_url.as_ref(), "https://api.nyx.nodes.guru/",);
  }
}
