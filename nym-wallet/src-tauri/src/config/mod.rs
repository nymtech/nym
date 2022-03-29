// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{error::BackendError, network::Network as WalletNetwork};
use config::defaults::{all::SupportedNetworks, ValidatorDetails};
use config::NymConfig;
use core::fmt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{fs, io, path::PathBuf};
use strum::IntoEnumIterator;
use url::Url;

const REMOTE_SOURCE_OF_VALIDATOR_URLS: &str =
  "https://nymtech.net/.wellknown/wallet/validators.json";

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
  // Base configuration is not part of the configuration file as it's not intended to be changed.
  #[serde(skip)]
  base: Base,

  // Network level configuration
  network: OptionalValidators,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Base {
  /// Information on all the networks that the wallet connects to.
  networks: SupportedNetworks,

  /// Validators that have been fetched dynamically, probably during startup.
  fetched_validators: OptionalValidators,
}

impl Default for Base {
  fn default() -> Self {
    let networks = WalletNetwork::iter().map(Into::into).collect();
    Base {
      networks: SupportedNetworks::new(networks),
      fetched_validators: OptionalValidators::default(),
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
  /// Get the available validators in the order
  /// 1. from the configuration file
  /// 2. provided remotely
  /// 3. hardcoded fallback
  pub fn get_validators(&self, network: WalletNetwork) -> impl Iterator<Item = ValidatorUrl> + '_ {
    // The base validators are (currently) stored as strings
    let base_validators = self.base.networks.validators(network.into()).map(|v| {
      v.clone()
        .try_into()
        .expect("The hardcoded validators are assumed to be valid urls")
    });

    self
      .base
      .fetched_validators
      .validators(network)
      .chain(self.network.validators(network))
      .cloned()
      .chain(base_validators)
      .unique()
  }

  pub fn get_nymd_urls(&self, network: WalletNetwork) -> impl Iterator<Item = Url> + '_ {
    self.get_validators(network).into_iter().map(|v| v.nymd_url)
  }

  pub fn get_api_urls(&self, network: WalletNetwork) -> impl Iterator<Item = Url> + '_ {
    self
      .get_validators(network)
      .into_iter()
      .filter_map(|v| v.api_url)
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

  pub async fn fetch_updated_validator_urls(&mut self) -> Result<(), BackendError> {
    let client = reqwest::Client::builder()
      .timeout(Duration::from_secs(3))
      .build()?;
    log::debug!(
      "Fetching validator urls from: {}",
      REMOTE_SOURCE_OF_VALIDATOR_URLS
    );
    let response = client
      .get(REMOTE_SOURCE_OF_VALIDATOR_URLS.to_string())
      .send()
      .await?;
    self.base.fetched_validators = serde_json::from_str(&response.text().await?)?;
    log::debug!("Received validator urls: \n{}", self.base.fetched_validators);
    Ok(())
  }
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

impl fmt::Display for ValidatorUrl {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s1 = format!("nymd_url: {}", self.nymd_url);
    let s2 = self.api_url.as_ref().map(|url| format!(", api_url: {}", url));
    write!(f, "    {}{},", s1, s2.unwrap_or_default())
  }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct OptionalValidators {
  // User supplied additional validator urls in addition to the hardcoded ones.
  // These are separate fields, rather than a map, to force the serialization order.
  mainnet: Option<Vec<ValidatorUrl>>,
  sandbox: Option<Vec<ValidatorUrl>>,
  qa: Option<Vec<ValidatorUrl>>,
}

impl OptionalValidators {
  fn validators(&self, network: WalletNetwork) -> impl Iterator<Item = &ValidatorUrl> {
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
    Config {
      base: Base::default(),
      network: OptionalValidators {
        mainnet: Some(vec![
          ValidatorDetails {
            nymd_url: "https://foo".to_string(),
            api_url: None,
          }
          .try_into()
          .unwrap(),
          ValidatorUrl {
            nymd_url: "https://baz".parse().unwrap(),
            api_url: Some("https://baz/api".parse().unwrap()),
          },
        ]),
        sandbox: Some(vec![ValidatorUrl {
          nymd_url: "https://bar".parse().unwrap(),
          api_url: Some("https://bar/api".parse().unwrap()),
        }]),
        qa: None,
      },
    }
  }

  #[test]
  fn serialize_to_toml() {
    assert_eq!(
      toml::to_string_pretty(&test_config()).unwrap(),
      r#"[[network.mainnet]]
nymd_url = 'https://foo/'

[[network.mainnet]]
nymd_url = 'https://baz/'
api_url = 'https://baz/api'

[[network.sandbox]]
nymd_url = 'https://bar/'
api_url = 'https://bar/api'
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

  #[test]
  fn get_urls_parsed_from_config() {
    let config = test_config();

    let nymd_url = config
      .get_validators(WalletNetwork::MAINNET)
      .next()
      .map(|v| v.nymd_url)
      .unwrap();
    assert_eq!(nymd_url.as_ref(), "https://foo/");

    // The first entry is missing an API URL
    let api_url = config
      .get_validators(WalletNetwork::MAINNET)
      .next()
      .and_then(|v| v.api_url);
    assert_eq!(api_url, None);
  }

  #[test]
  fn get_urls_from_defaults() {
    let config = Config::default();

    let nymd_url = config
      .get_validators(WalletNetwork::MAINNET)
      .next()
      .map(|v| v.nymd_url)
      .unwrap();
    assert_eq!(nymd_url.as_ref(), "https://rpc.nyx.nodes.guru/");

    let api_url = config
      .get_validators(WalletNetwork::MAINNET)
      .next()
      .and_then(|v| v.api_url)
      .unwrap();
    assert_eq!(api_url.as_ref(), "https://api.nyx.nodes.guru/",);
  }
}
