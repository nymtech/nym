// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::fmt;
use std::collections::HashMap;
use std::str::FromStr;
use std::{fs, io, path::PathBuf};

use itertools::Itertools;
use nym_validator_client::nyxd::AccountId as CosmosAccountId;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use url::Url;

use nym_config::defaults::{DenomDetailsOwned, NymNetworkDetails, ValidatorDetails};
use nym_wallet_types::network::Network as WalletNetwork;
use nym_wallet_types::network_config;

use crate::error::BackendError;
use crate::platform_constants::{CONFIG_DIR_NAME, CONFIG_FILENAME};

pub const REMOTE_SOURCE_OF_NYXD_URLS: &str =
    "https://nymtech.net/.wellknown/wallet/validators.json";

const CURRENT_GLOBAL_CONFIG_VERSION: u32 = 1;
const CURRENT_NETWORK_CONFIG_VERSION: u32 = 1;
pub(crate) const CUSTOM_SIMULATED_GAS_MULTIPLIER: f32 = 1.5;

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
    selected_nyxd_url: Option<Url>,
    // Default nyxd URL assigned during login, can be used when the user wants
    // to revert back its selected validator URL
    default_nyxd_url: Option<Url>,

    selected_api_url: Option<Url>,

    // Additional user provided validators.
    // It is an option for the purpose of file serialization.
    nyxd_urls: Option<Vec<ValidatorConfigEntry>>,
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
            default_nyxd_url: None,
            selected_nyxd_url: None,
            selected_api_url: None,
            nyxd_urls: None,
        }
    }
}

impl NetworkConfig {
    fn validators(&self) -> impl Iterator<Item = &ValidatorConfigEntry> {
        self.nyxd_urls.iter().flat_map(|v| v.iter())
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
            let network = match WalletNetwork::from_str(network) {
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
                    log::trace!("Not loading {:#?}: {err}", file);
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
                Err(err) => log::trace!("Not loading {:#?}: {err}", file),
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
    ) -> impl Iterator<Item = ValidatorConfigEntry> + '_ {
        self.base.networks.validators(&network).map(|v| {
            v.clone()
                .try_into()
                .expect("The hardcoded validators are assumed to be valid urls")
        })
    }

    pub fn get_configured_validators(
        &self,
        network: WalletNetwork,
    ) -> impl Iterator<Item = ValidatorConfigEntry> + '_ {
        self.networks
            .get(&network.as_key())
            .into_iter()
            .flat_map(|c| c.validators().cloned())
    }

    pub fn get_mixnet_contract_address(&self, network: WalletNetwork) -> CosmosAccountId {
        self.base
            .networks
            .mixnet_contract_address(&network)
            .expect("No mixnet contract address found in config")
            .parse()
            .expect("Wrong format for mixnet contract address")
    }

    pub fn get_vesting_contract_address(&self, network: WalletNetwork) -> CosmosAccountId {
        self.base
            .networks
            .vesting_contract_address(&network)
            .expect("No vesting contract address found in config")
            .parse()
            .expect("Wrong format for vesting contract address")
    }

    pub fn set_default_nyxd_urls(&mut self, urls: &HashMap<WalletNetwork, Url>) {
        for (network, url) in urls {
            self.set_default_nyxd_url(url.to_owned(), network);
        }
    }

    pub fn set_default_nyxd_url(&mut self, nyxd_url: Url, network: &WalletNetwork) {
        log::debug!(
            "set default nyxd URL for {network} {}",
            nyxd_url.to_string()
        );
        if let Some(net) = self.networks.get_mut(&network.as_key()) {
            net.default_nyxd_url = Some(nyxd_url);
        } else {
            self.networks.insert(
                network.as_key(),
                NetworkConfig {
                    default_nyxd_url: Some(nyxd_url),
                    ..NetworkConfig::default()
                },
            );
        }
    }

    pub fn select_nyxd_url(&mut self, nyxd_url: Url, network: WalletNetwork) {
        if let Some(net) = self.networks.get_mut(&network.as_key()) {
            net.selected_nyxd_url = Some(nyxd_url);
        } else {
            self.networks.insert(
                network.as_key(),
                NetworkConfig {
                    selected_nyxd_url: Some(nyxd_url),
                    ..NetworkConfig::default()
                },
            );
        }
    }

    pub fn reset_nyxd_url(&mut self, network: WalletNetwork) {
        match self.networks.get_mut(&network.as_key()) {
            Some(net) => net.selected_nyxd_url = None,
            None => log::warn!("reset_nyxd_url: {network} network not found, ignoring"),
        }
    }

    pub fn select_nym_api_url(&mut self, api_url: Url, network: WalletNetwork) {
        if let Some(net) = self.networks.get_mut(&network.as_key()) {
            net.selected_api_url = Some(api_url);
        } else {
            self.networks.insert(
                network.as_key(),
                NetworkConfig {
                    selected_api_url: Some(api_url),
                    ..NetworkConfig::default()
                },
            );
        }
    }

    pub fn get_selected_validator_nyxd_url(&self, network: WalletNetwork) -> Option<Url> {
        self.networks.get(&network.as_key()).and_then(|config| {
            log::debug!(
                "get selected nyxd url for {} {:?}",
                network.to_string(),
                config.selected_nyxd_url,
            );
            config.selected_nyxd_url.clone()
        })
    }

    pub fn get_default_nyxd_url(&self, network: WalletNetwork) -> Option<Url> {
        self.networks.get(&network.as_key()).and_then(|config| {
            log::debug!(
                "get default nyxd url for {} {:?}",
                network.to_string(),
                config.default_nyxd_url,
            );
            config.default_nyxd_url.clone()
        })
    }

    pub fn get_selected_nym_api_url(&self, network: &WalletNetwork) -> Option<Url> {
        self.networks
            .get(&network.as_key())
            .and_then(|config| config.selected_api_url.clone())
    }

    pub fn add_validator_url(&mut self, url: ValidatorConfigEntry, network: WalletNetwork) {
        if let Some(network_config) = self.networks.get_mut(&network.as_key()) {
            if let Some(ref mut urls) = network_config.nyxd_urls {
                urls.push(url);
            } else {
                network_config.nyxd_urls = Some(vec![url]);
            }
        } else {
            self.networks.insert(
                network.as_key(),
                NetworkConfig {
                    nyxd_urls: Some(vec![url]),
                    ..NetworkConfig::default()
                },
            );
        }
    }

    pub fn remove_validator_url(&mut self, url: ValidatorConfigEntry, network: WalletNetwork) {
        if let Some(network_config) = self.networks.get_mut(&network.as_key()) {
            if let Some(ref mut urls) = network_config.nyxd_urls {
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
pub struct ValidatorConfigEntry {
    pub nyxd_url: Url,
    pub nyxd_name: Option<String>,
    pub api_url: Option<Url>,
}

impl TryFrom<ValidatorDetails> for ValidatorConfigEntry {
    type Error = BackendError;

    fn try_from(validator: ValidatorDetails) -> Result<Self, Self::Error> {
        Ok(ValidatorConfigEntry {
            nyxd_url: validator.nyxd_url.parse()?,
            nyxd_name: None,
            api_url: match &validator.api_url {
                Some(url) => Some(url.parse()?),
                None => None,
            },
        })
    }
}

impl TryFrom<network_config::Validator> for ValidatorConfigEntry {
    type Error = BackendError;

    fn try_from(validator: network_config::Validator) -> Result<Self, Self::Error> {
        Ok(ValidatorConfigEntry {
            nyxd_url: validator.nyxd_url.parse()?,
            nyxd_name: validator.nyxd_name,
            api_url: match &validator.api_url {
                Some(url) => Some(url.parse()?),
                None => None,
            },
        })
    }
}

impl fmt::Display for ValidatorConfigEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s1 = format!("nyxd_url: {}", self.nyxd_url);
        let name = self.nyxd_name.as_ref().map(|name| format!(" ({name})"));
        let s2 = self.api_url.as_ref().map(|url| format!(", api_url: {url}"));
        write!(
            f,
            "    {}{}{},",
            s1,
            name.unwrap_or_default(),
            s2.unwrap_or_default()
        )
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptionalValidators {
    // User supplied additional validator urls in addition to the hardcoded ones.
    // These are separate fields, rather than a map, to force the serialization order.
    mainnet: Option<Vec<ValidatorConfigEntry>>,
    sandbox: Option<Vec<ValidatorConfigEntry>>,
    qa: Option<Vec<ValidatorConfigEntry>>,
}

impl OptionalValidators {
    pub fn validators(
        &self,
        network: WalletNetwork,
    ) -> impl Iterator<Item = &ValidatorConfigEntry> {
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
        write!(f, "{s1}{s2}{s3}")
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
struct SupportedNetworks {
    networks: HashMap<WalletNetwork, NetworkDetails>,
}

impl SupportedNetworks {
    fn new(support: Vec<WalletNetwork>) -> Self {
        SupportedNetworks {
            networks: support
                .into_iter()
                .map(|n| {
                    let details = NetworkDetails::from(NymNetworkDetails::from(n));
                    (n, details)
                })
                .collect(),
        }
    }

    fn mixnet_contract_address(&self, network: &WalletNetwork) -> Option<&str> {
        self.networks
            .get(network)
            .map(|network_details| network_details.mixnet_contract_address.as_str())
    }

    fn vesting_contract_address(&self, network: &WalletNetwork) -> Option<&str> {
        self.networks
            .get(network)
            .map(|network_details| network_details.vesting_contract_address.as_str())
    }

    fn validators(&self, network: &WalletNetwork) -> impl Iterator<Item = &ValidatorDetails> {
        self.networks
            .get(network)
            .map(|network_details| &network_details.validators)
            .into_iter()
            .flatten()
    }
}

// Simplified variant of NymNetworkDetails for serialization to config file
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct NetworkDetails {
    bech32_prefix: String,
    mix_denom: DenomDetailsOwned,
    stake_denom: DenomDetailsOwned,
    mixnet_contract_address: String,
    vesting_contract_address: String,
    validators: Vec<ValidatorDetails>,
}

// Possibly a bit naff, but WalletNetwork is converted into the more general NymNetworkDetails, which here
// is converted to the format specific for serialization to config
impl From<NymNetworkDetails> for NetworkDetails {
    fn from(details: NymNetworkDetails) -> Self {
        NetworkDetails {
            bech32_prefix: details.chain_details.bech32_account_prefix,
            mix_denom: details.chain_details.mix_denom,
            stake_denom: details.chain_details.stake_denom,
            mixnet_contract_address: details
                .contracts
                .mixnet_contract_address
                .unwrap_or_default(),
            vesting_contract_address: details
                .contracts
                .vesting_contract_address
                .unwrap_or_default(),
            validators: details.endpoints,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        let netconfig = NetworkConfig {
            selected_nyxd_url: None,
            selected_api_url: Some("https://my_api_url.com".parse().unwrap()),

            nyxd_urls: Some(vec![
                ValidatorConfigEntry {
                    nyxd_url: "https://foo".parse().unwrap(),
                    nyxd_name: Some("FooName".to_string()),
                    api_url: None,
                },
                ValidatorConfigEntry {
                    nyxd_url: "https://bar".parse().unwrap(),
                    nyxd_name: None,
                    api_url: Some("https://bar/api".parse().unwrap()),
                },
                ValidatorConfigEntry {
                    nyxd_url: "https://baz".parse().unwrap(),
                    nyxd_name: None,
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

[[nyxd_urls]]
nyxd_url = 'https://foo/'
nyxd_name = 'FooName'

[[nyxd_urls]]
nyxd_url = 'https://bar/'
api_url = 'https://bar/api'

[[nyxd_urls]]
nyxd_url = 'https://baz/'
api_url = 'https://baz/api'
"#
        );
    }

    #[test]
    fn serialize_to_json() {
        let config = test_config();
        let netconfig = &config.networks[&WalletNetwork::MAINNET.as_key()];
        println!("{}", serde_json::to_string_pretty(netconfig).unwrap());
        assert_eq!(
            serde_json::to_string_pretty(netconfig).unwrap(),
            r#"{
  "version": 1,
  "selected_nyxd_url": null,
  "default_nyxd_url": null,
  "selected_api_url": "https://my_api_url.com/",
  "nyxd_urls": [
    {
      "nyxd_url": "https://foo/",
      "nyxd_name": "FooName",
      "api_url": null
    },
    {
      "nyxd_url": "https://bar/",
      "nyxd_name": null,
      "api_url": "https://bar/api"
    },
    {
      "nyxd_url": "https://baz/",
      "nyxd_name": null,
      "api_url": "https://baz/api"
    }
  ]
}"#
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

        let nyxd_url = config
            .get_configured_validators(WalletNetwork::MAINNET)
            .next()
            .map(|v| v.nyxd_url)
            .unwrap();
        assert_eq!(nyxd_url.as_ref(), "https://foo/");

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

        let nyxd_url = config
            .get_base_validators(WalletNetwork::MAINNET)
            .next()
            .map(|v| v.nyxd_url)
            .unwrap();
        assert_eq!(nyxd_url.as_ref(), "https://rpc.nymtech.net/");

        let api_url = config
            .get_base_validators(WalletNetwork::MAINNET)
            .next()
            .and_then(|v| v.api_url)
            .unwrap();
        assert_eq!(api_url.as_ref(), "https://validator.nymtech.net/api/",);
    }
}
