// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{error::BackendError, network::Network as WalletNetwork};
use config::defaults::{all::SupportedNetworks, ValidatorDetails};
use config::NymConfig;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::iter::zip;
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
  pub fn get_validators(
    &self,
    network: WalletNetwork,
  ) -> impl Iterator<Item = &ValidatorDetails> + '_ {
    self
      .base
      .fetched_validators
      .validators(network)
      .chain(self.network.validators(network))
      .chain(self.base.networks.validators(network.into()))
  }

  pub fn get_validators_with_api_endpoint(
    &self,
    network: WalletNetwork,
  ) -> impl Iterator<Item = ValidatorWithApiEndpoint> + '_ {
    self
      .get_validators(network)
      .filter_map(|validator| ValidatorWithApiEndpoint::try_from(validator.clone()).ok())
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
    let response = client
      .get(REMOTE_SOURCE_OF_VALIDATOR_URLS.to_string())
      .send()
      .await?;
    self.base.fetched_validators = serde_json::from_str(&response.text().await?)?;
    Ok(())
  }

  pub async fn check_validator_health(
    &self,
    network: WalletNetwork,
  ) -> Result<Vec<(ValidatorDetails, StatusCode)>, BackendError> {
    // Limit the number of validators we query
    let max_validators = 200_usize;
    let validators_to_query = || self.get_validators(network).take(max_validators).cloned();

    let validator_urls = validators_to_query().map(|v| {
      let mut health_url = v.nymd_url();
      health_url.set_path("health");
      (v, health_url)
    });

    let client = reqwest::Client::builder()
      .timeout(Duration::from_secs(3))
      .build()?;

    let requests = validator_urls.map(|(_, url)| client.get(url).send());
    let responses = futures::future::join_all(requests).await;

    let validators_responding_success =
      zip(validators_to_query(), responses).filter_map(|(v, r)| match r {
        Ok(r) if r.status().is_success() => Some((v, r.status())),
        _ => None,
      });

    Ok(validators_responding_success.collect::<Vec<_>>())
  }

  #[allow(unused)]
  pub async fn check_validator_health_for_all_networks(
    &self,
  ) -> Result<HashMap<WalletNetwork, Vec<(ValidatorDetails, StatusCode)>>, BackendError> {
    let validator_health_requests =
      WalletNetwork::iter().map(|network| self.check_validator_health(network));

    let responses_keyed_by_network = zip(
      WalletNetwork::iter(),
      futures::future::join_all(validator_health_requests).await,
    );

    // Iterate and collect manually to be able to return errors in the response
    let mut responses = HashMap::new();
    for (network, response) in responses_keyed_by_network {
      responses.insert(network, response?);
    }
    Ok(responses)
  }
}

// Unlike `ValidatorDetails` this represents validators which are always supposed to have a
// validator-api endpoint running.
#[derive(Clone, Debug)]
pub struct ValidatorWithApiEndpoint {
  pub nymd_url: Url,
  pub api_url: Url,
}

impl TryFrom<ValidatorDetails> for ValidatorWithApiEndpoint {
  type Error = BackendError;

  fn try_from(validator: ValidatorDetails) -> Result<Self, Self::Error> {
    match validator.api_url() {
      Some(api_url) => Ok(ValidatorWithApiEndpoint {
        nymd_url: validator.nymd_url(),
        api_url,
      }),
      None => Err(BackendError::NoValidatorApiUrlConfigured),
    }
  }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct OptionalValidators {
  // User supplied additional validator urls in addition to the hardcoded ones.
  // These are separate fields, rather than a map, to force the serialization order.
  mainnet: Option<Vec<ValidatorDetails>>,
  sandbox: Option<Vec<ValidatorDetails>>,
  qa: Option<Vec<ValidatorDetails>>,
}

impl OptionalValidators {
  fn validators(&self, network: WalletNetwork) -> impl Iterator<Item = &ValidatorDetails> {
    match network {
      WalletNetwork::MAINNET => self.mainnet.as_ref(),
      WalletNetwork::SANDBOX => self.sandbox.as_ref(),
      WalletNetwork::QA => self.qa.as_ref(),
    }
    .into_iter()
    .flatten()
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
          },
          ValidatorDetails {
            nymd_url: "https://baz".to_string(),
            api_url: Some("https://baz/api".to_string()),
          },
        ]),
        sandbox: Some(vec![ValidatorDetails {
          nymd_url: "https://bar".to_string(),
          api_url: Some("https://bar/api".to_string()),
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
nymd_url = 'https://foo'

[[network.mainnet]]
nymd_url = 'https://baz'
api_url = 'https://baz/api'

[[network.sandbox]]
nymd_url = 'https://bar'
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
      .map(ValidatorDetails::nymd_url)
      .unwrap();
    assert_eq!(nymd_url.to_string(), "https://foo/".to_string());

    // The first entry is missing an API URL
    let api_url = config
      .get_validators(WalletNetwork::MAINNET)
      .next()
      .and_then(ValidatorDetails::api_url);
    assert_eq!(api_url, None);
  }

  #[test]
  fn get_urls_from_defaults() {
    let config = Config::default();

    let nymd_url = config
      .get_validators(WalletNetwork::MAINNET)
      .next()
      .map(ValidatorDetails::nymd_url)
      .unwrap();
    assert_eq!(
      nymd_url.to_string(),
      "https://rpc.nyx.nodes.guru/".to_string()
    );

    let api_url = config
      .get_validators(WalletNetwork::MAINNET)
      .next()
      .and_then(ValidatorDetails::api_url)
      .unwrap();
    assert_eq!(
      api_url.to_string(),
      "https://api.nyx.nodes.guru/".to_string()
    );
  }
}
