use crate::config;
use crate::error::BackendError;
use nym_wallet_types::network::Network;
use nym_wallet_types::network_config;

use strum::IntoEnumIterator;
use validator_client::nymd::{AccountId as CosmosAccountId, SigningNymdClient};
use validator_client::Client;

use itertools::Itertools;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use url::Url;

// use nym_types::currency::{CoinMetadata, Denom};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// Some hardcoded metadata overrides
static METADATA_OVERRIDES: Lazy<Vec<(Url, ValidatorMetadata)>> = Lazy::new(|| {
    vec![(
        "https://rpc.nyx.nodes.guru/".parse().unwrap(),
        ValidatorMetadata {
            name: Some("Nodes.Guru".to_string()),
        },
    )]
});

#[tauri::command]
pub async fn load_config_from_files(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    state.write().await.load_config_files();
    Ok(())
}

#[tauri::command]
pub async fn save_config_to_files(
    state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
    state.read().await.save_config_files()
}

#[derive(Default)]
pub struct State {
    config: config::Config,
    signing_clients: HashMap<Network, Client<SigningNymdClient>>,
    current_network: Network,

    // All the accounts the we get from decrypting the wallet. We hold on to these for being able to
    // switch accounts on-the-fly
    all_accounts: Vec<WalletAccountIds>,

    /// Validators that have been fetched dynamically, probably during startup.
    fetched_validators: config::OptionalValidators,

    /// We fetch (and cache) some metadata, such as names, when available
    validator_metadata: HashMap<Url, ValidatorMetadata>,
    // registered_coins: HashMap<Network, HashMap<Denom, CoinMetadata>>,
}

pub(crate) struct WalletAccountIds {
    // The wallet account id
    pub id: crate::wallet_storage::AccountId,
    // The set of corresponding network identities derived from the mnemonic
    pub addresses: HashMap<Network, CosmosAccountId>,
}

impl State {
    pub fn client(&self, network: Network) -> Result<&Client<SigningNymdClient>, BackendError> {
        self.signing_clients
            .get(&network)
            .ok_or(BackendError::ClientNotInitialized)
    }

    pub fn client_mut(
        &mut self,
        network: Network,
    ) -> Result<&mut Client<SigningNymdClient>, BackendError> {
        self.signing_clients
            .get_mut(&network)
            .ok_or(BackendError::ClientNotInitialized)
    }

    pub fn current_client(&self) -> Result<&Client<SigningNymdClient>, BackendError> {
        self.signing_clients
            .get(&self.current_network)
            .ok_or(BackendError::ClientNotInitialized)
    }

    #[allow(unused)]
    pub fn current_client_mut(&mut self) -> Result<&mut Client<SigningNymdClient>, BackendError> {
        self.signing_clients
            .get_mut(&self.current_network)
            .ok_or(BackendError::ClientNotInitialized)
    }

    pub fn config(&self) -> &config::Config {
        &self.config
    }

    /// Load configuration from files. If unsuccessful we just log it and move on.
    pub fn load_config_files(&mut self) {
        self.config = config::Config::load_from_files();
    }

    #[allow(unused)]
    pub fn save_config_files(&self) -> Result<(), BackendError> {
        Ok(self.config.save_to_files()?)
    }

    pub fn add_client(&mut self, network: Network, client: Client<SigningNymdClient>) {
        self.signing_clients.insert(network, client);
    }

    pub fn set_network(&mut self, network: Network) {
        self.current_network = network;
    }

    pub fn current_network(&self) -> Network {
        self.current_network
    }

    pub(crate) fn set_all_accounts(&mut self, all_accounts: Vec<WalletAccountIds>) {
        self.all_accounts = all_accounts
    }

    pub(crate) fn get_all_accounts(&self) -> impl Iterator<Item = &WalletAccountIds> {
        self.all_accounts.iter()
    }

    pub fn logout(&mut self) {
        self.signing_clients = HashMap::new();
    }

    /// Get the available validators in the order
    /// 1. from the configuration file
    /// 2. provided remotely
    /// 3. hardcoded fallback
    /// The format is the config backend format, which is flat due to serialization preference.
    pub fn get_config_validator_entries(
        &self,
        network: Network,
    ) -> impl Iterator<Item = config::ValidatorConfigEntry> + '_ {
        let validators_in_config = self.config.get_configured_validators(network);
        let fetched_validators = self.fetched_validators.validators(network).cloned();
        let default_validators = self.config.get_base_validators(network);

        // All the validators, in decending list of priority
        let validators = validators_in_config
            .chain(fetched_validators)
            .chain(default_validators)
            .unique_by(|v| (v.nymd_url.clone(), v.api_url.clone()));

        // Annotate with dynamic metadata
        validators.map(|v| {
            let metadata = self.validator_metadata.get(&v.nymd_url);
            let name = v
                .nymd_name
                .or_else(|| metadata.and_then(|m| m.name.clone()));
            config::ValidatorConfigEntry {
                nymd_url: v.nymd_url,
                nymd_name: name,
                api_url: v.api_url,
            }
        })
    }

    pub fn get_nymd_urls_only(&self, network: Network) -> impl Iterator<Item = Url> + '_ {
        self.get_config_validator_entries(network)
            .into_iter()
            .map(|v| v.nymd_url)
    }

    pub fn get_api_urls_only(&self, network: Network) -> impl Iterator<Item = Url> + '_ {
        self.get_config_validator_entries(network)
            .into_iter()
            .filter_map(|v| v.api_url)
    }

    /// Get the list of validator nymd urls in the network config format, suitable for passing on to
    /// the UI
    pub fn get_nymd_urls(
        &self,
        network: Network,
    ) -> impl Iterator<Item = network_config::ValidatorUrl> + '_ {
        self.get_config_validator_entries(network)
            .into_iter()
            .map(|v| network_config::ValidatorUrl {
                url: v.nymd_url.to_string(),
                name: v.nymd_name,
            })
    }

    /// Get the list of validator-api urls in the network config format, suitable for passing on to
    /// the UI
    pub fn get_api_urls(
        &self,
        network: Network,
    ) -> impl Iterator<Item = network_config::ValidatorUrl> + '_ {
        self.get_config_validator_entries(network)
            .into_iter()
            .filter_map(|v| {
                v.api_url.map(|u| network_config::ValidatorUrl {
                    url: u.to_string(),
                    name: None,
                })
            })
    }

    pub fn get_all_nymd_urls(&self) -> HashMap<Network, Vec<Url>> {
        Network::iter()
            .flat_map(|network| {
                self.get_nymd_urls_only(network)
                    .map(move |url| (network, url))
            })
            .into_group_map()
    }

    pub fn get_all_api_urls(&self) -> HashMap<Network, Vec<Url>> {
        Network::iter()
            .flat_map(|network| {
                self.get_api_urls_only(network)
                    .map(move |url| (network, url))
            })
            .into_group_map()
    }

    /// Fetch validator urls remotely. These are used to in addition to the base ones, and the user
    /// configured ones.
    pub async fn fetch_updated_validator_urls(&mut self) -> Result<(), BackendError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        log::debug!(
            "Fetching validator urls from: {}",
            crate::config::REMOTE_SOURCE_OF_VALIDATOR_URLS
        );
        let response = client
            .get(crate::config::REMOTE_SOURCE_OF_VALIDATOR_URLS.to_string())
            .send()
            .await?;

        self.fetched_validators = serde_json::from_str(&response.text().await?)?;
        log::debug!("Received validator urls: \n{}", self.fetched_validators);

        self.refresh_validator_status().await?;

        Ok(())
    }

    pub async fn refresh_validator_status(&mut self) -> Result<(), BackendError> {
        log::debug!("Refreshing validator status");

        // All urls for all networks
        let nymd_urls = self
            .get_all_nymd_urls()
            .into_iter()
            .flat_map(|(_, urls)| urls.into_iter());

        // Fetch status for all urls
        let responses = fetch_status_for_urls(nymd_urls).await?;

        // Update the stored metadata
        self.apply_responses(responses)?;

        // Override some overrides for usability
        self.apply_metadata_override(METADATA_OVERRIDES.to_vec());

        Ok(())
    }

    fn apply_responses(
        &mut self,
        responses: Vec<Result<(Url, String), reqwest::Error>>,
    ) -> Result<(), BackendError> {
        for response in responses.into_iter().flatten() {
            let json: serde_json::Value = serde_json::from_str(&response.1)?;
            let moniker = &json["result"]["node_info"]["moniker"];
            log::debug!("Fetched moniker for: {}: {}", response.0, moniker);

            // Insert into metadata map
            if let Some(ref mut m) = self.validator_metadata.get_mut(&response.0) {
                m.name = Some(moniker.to_string());
            } else {
                self.validator_metadata.insert(
                    response.0,
                    ValidatorMetadata {
                        name: Some(moniker.to_string()),
                    },
                );
            }
        }
        Ok(())
    }

    fn apply_metadata_override(&mut self, metadata_overrides: Vec<(Url, ValidatorMetadata)>) {
        for (url, metadata) in metadata_overrides {
            log::debug!("Overriding (some) metadata for: {url}");
            if let Some(m) = self.validator_metadata.get_mut(&url) {
                m.name = metadata.name;
            } else {
                self.validator_metadata.insert(url, metadata);
            }
        }
    }

    pub fn select_validator_nymd_url(
        &mut self,
        url: &str,
        network: Network,
    ) -> Result<(), BackendError> {
        self.config.select_validator_nymd_url(url.parse()?, network);
        if let Ok(client) = self.client_mut(network) {
            client.change_nymd(url.parse()?)?;
        }
        Ok(())
    }

    pub fn select_validator_api_url(
        &mut self,
        url: &str,
        network: Network,
    ) -> Result<(), BackendError> {
        self.config.select_validator_api_url(url.parse()?, network);
        if let Ok(client) = self.client_mut(network) {
            client.change_validator_api(url.parse()?);
        }
        Ok(())
    }

    pub fn add_validator_url(&mut self, url: config::ValidatorConfigEntry, network: Network) {
        self.config.add_validator_url(url, network);
    }

    pub fn remove_validator_url(&mut self, url: config::ValidatorConfigEntry, network: Network) {
        self.config.remove_validator_url(url, network)
    }
}

async fn fetch_status_for_urls(
    nymd_urls: impl Iterator<Item = Url>,
) -> Result<Vec<Result<(Url, String), reqwest::Error>>, BackendError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;

    let responses = futures::future::join_all(nymd_urls.into_iter().map(|url| {
        let client = &client;
        let status_url = url.join("status").unwrap_or_else(|_| url.clone());
        async move {
            let resp = client.get(status_url).send().await?;
            resp.text().await.map(|text| (url, text))
        }
    }))
    .await;

    Ok(responses)
}

// Validator metadata that can by dynamically populated
#[derive(Clone, Debug)]
pub struct ValidatorMetadata {
    pub name: Option<String>,
}

#[macro_export]
macro_rules! client {
    ($state:ident) => {
        $state.read().await.current_client()?
    };
}

#[macro_export]
macro_rules! nymd_client {
    ($state:ident) => {
        $state.read().await.current_client()?.nymd
    };
}

#[macro_export]
macro_rules! api_client {
    ($state:ident) => {
        $state.read().await.current_client()?.validator_api
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_validators_urls_prepends() {
        let mut state = State::default();
        let _api_urls = state.get_api_urls(Network::MAINNET).collect::<Vec<_>>();

        state.add_validator_url(
            config::ValidatorConfigEntry {
                nymd_url: "http://nymd_url.com".parse().unwrap(),
                nymd_name: Some("NymdUrl".to_string()),
                api_url: Some("http://nymd_url.com/api".parse().unwrap()),
            },
            Network::MAINNET,
        );

        state.add_validator_url(
            config::ValidatorConfigEntry {
                nymd_url: "http://foo.com".parse().unwrap(),
                nymd_name: None,
                api_url: None,
            },
            Network::MAINNET,
        );

        state.add_validator_url(
            config::ValidatorConfigEntry {
                nymd_url: "http://bar.com".parse().unwrap(),
                nymd_name: None,
                api_url: None,
            },
            Network::MAINNET,
        );

        assert_eq!(
            state
                .get_nymd_urls_only(Network::MAINNET)
                .collect::<Vec<_>>(),
            vec![
                "http://nymd_url.com/".parse().unwrap(),
                "http://foo.com".parse().unwrap(),
                "http://bar.com".parse().unwrap(),
                "https://rpc.nyx.nodes.guru".parse().unwrap(),
            ],
        );
        assert_eq!(
            state
                .get_api_urls_only(Network::MAINNET)
                .collect::<Vec<_>>(),
            vec![
                "http://nymd_url.com/api".parse().unwrap(),
                "https://validator.nymtech.net/api/".parse().unwrap(),
            ],
        );
        assert_eq!(
            state
                .get_all_nymd_urls()
                .get(&Network::MAINNET)
                .unwrap()
                .clone(),
            vec![
                "http://nymd_url.com/".parse().unwrap(),
                "http://foo.com".parse().unwrap(),
                "http://bar.com".parse().unwrap(),
                "https://rpc.nyx.nodes.guru".parse().unwrap(),
            ],
        )
    }
}
