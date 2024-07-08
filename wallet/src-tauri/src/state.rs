use crate::config;
use crate::error::BackendError;
use crate::simulate::SimulateResult;
use ::nym_config::defaults::NymNetworkDetails;
use cosmwasm_std::Decimal;
use itertools::Itertools;
use log::warn;
use nym_types::currency::{DecCoin, Denom, RegisteredCoins};
use nym_types::fees::FeeDetails;
use nym_validator_client::nyxd::cosmwasm_client::types::SimulateResponse;
use nym_validator_client::nyxd::{AccountId as CosmosAccountId, Coin, Fee, SigningCosmWasmClient};
use nym_validator_client::DirectSigningHttpRpcValidatorClient;
use nym_wallet_types::network::Network;
use nym_wallet_types::network_config;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use url::Url;

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
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    state.write().await.load_config_files();
    Ok(())
}

#[tauri::command]
pub async fn save_config_to_files(
    state: tauri::State<'_, WalletState>,
) -> Result<(), BackendError> {
    state.read().await.save_config_files()
}

#[derive(Default, Clone)]
pub struct WalletState {
    inner: Arc<RwLock<WalletStateInner>>,
}

impl WalletState {
    // not the best API, but those are exposed here for backwards compatibility with the existing
    // state type assumptions so that we wouldn't need to fix it up everywhere at once
    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, WalletStateInner> {
        self.inner.read().await
    }

    pub(crate) async fn write(&self) -> RwLockWriteGuard<'_, WalletStateInner> {
        self.inner.write().await
    }
}

#[derive(Default)]
pub struct WalletStateInner {
    config: config::Config,
    signing_clients: HashMap<Network, DirectSigningHttpRpcValidatorClient>,
    current_network: Network,

    // All the accounts the we get from decrypting the wallet. We hold on to these for being able to
    // switch accounts on-the-fly
    all_accounts: Vec<WalletAccountIds>,

    /// Validators that have been fetched dynamically, probably during startup.
    fetched_validators: config::OptionalValidators,

    /// We fetch (and cache) some metadata, such as names, when available
    validator_metadata: HashMap<Url, ValidatorMetadata>,
    registered_coins: HashMap<Network, RegisteredCoins>,

    react_state: Option<String>,
}

pub(crate) struct WalletAccountIds {
    // The wallet account id
    pub id: crate::wallet_storage::AccountId,
    // The set of corresponding network identities derived from the mnemonic
    pub addresses: HashMap<Network, CosmosAccountId>,
}

impl WalletStateInner {
    pub fn get_react_state(&self) -> Result<Option<String>, BackendError> {
        Ok(self.react_state.clone())
    }

    pub fn set_react_state(&mut self, new_value: Option<String>) -> Result<(), BackendError> {
        self.react_state = new_value;
        Ok(())
    }

    pub fn attempt_convert_to_fixed_fee(&self, coin: DecCoin) -> Result<Fee, BackendError> {
        // first we have to convert the coin to its base denomination
        let base_coin = self.attempt_convert_to_base_coin(coin)?;

        // then we get the gas price for the current network
        let current_client = self.current_client()?;
        let gas_price = current_client.nyxd.gas_price();

        Ok(Fee::manual_with_gas_price(base_coin, gas_price.clone()))
    }

    // note that `Coin` is ALWAYS the base coin
    pub fn attempt_convert_to_base_coin(&self, coin: DecCoin) -> Result<Coin, BackendError> {
        let registered_coins = self
            .registered_coins
            .get(&self.current_network)
            .ok_or_else(|| BackendError::UnknownCoinDenom(coin.denom.clone()))?;

        Ok(registered_coins.attempt_convert_to_base_coin(coin)?)
    }

    pub fn attempt_convert_to_display_dec_coin(&self, coin: Coin) -> Result<DecCoin, BackendError> {
        let registered_coins = self
            .registered_coins
            .get(&self.current_network)
            .ok_or_else(|| BackendError::UnknownCoinDenom(coin.denom.clone()))?;

        Ok(registered_coins.attempt_convert_to_display_dec_coin(coin)?)
    }

    pub(crate) fn display_coin_from_base_decimal(
        &self,
        base_denom: &Denom,
        base_amount: Decimal,
    ) -> Result<DecCoin, BackendError> {
        let registered_coins = self
            .registered_coins
            .get(&self.current_network)
            .ok_or_else(|| BackendError::UnknownCoinDenom(base_denom.clone()))?;

        Ok(registered_coins
            .attempt_create_display_coin_from_base_dec_amount(base_denom, base_amount)?)
    }

    pub(crate) fn default_zero_mix_display_coin(&self) -> DecCoin {
        self.current_network.default_zero_mix_display_coin()
    }

    pub(crate) fn registered_coins(&self) -> Result<&RegisteredCoins, BackendError> {
        self.registered_coins
            .get(&self.current_network)
            .ok_or(BackendError::NoCoinsRegistered {
                network: self.current_network,
            })
    }

    pub(crate) fn convert_tx_fee(&self, fee: Option<&Fee>) -> Option<DecCoin> {
        let mut fee_amount = fee?.try_get_manual_amount()?;
        if fee_amount.len() > 1 {
            warn!(
            "our tx fee contained more than a single denomination. using the first one for display"
        )
        }
        if fee_amount.is_empty() {
            warn!("our tx has had an unknown fee set");
            None
        } else {
            self.attempt_convert_to_display_dec_coin(fee_amount.pop().unwrap())
                .ok()
        }
    }

    // this one is rather gnarly and I'm not 100% sure how to feel about existence of it
    pub(crate) fn create_detailed_fee(
        &self,
        simulate_response: SimulateResponse,
    ) -> Result<FeeDetails, BackendError> {
        // this MUST succeed as we just used it before
        let client = self.current_client()?;
        let gas_price = client.nyxd.gas_price().clone();
        let gas_adjustment = client.nyxd.simulated_gas_multiplier();

        let res = SimulateResult::new(simulate_response.gas_info, gas_price, gas_adjustment);

        let amount = res
            .to_fee_amount()
            .map(|amount| self.attempt_convert_to_display_dec_coin(amount.into()))
            .transpose()?;

        Ok(FeeDetails::new(amount, res.to_fee()))
    }

    pub fn client(
        &self,
        network: Network,
    ) -> Result<&DirectSigningHttpRpcValidatorClient, BackendError> {
        self.signing_clients
            .get(&network)
            .ok_or(BackendError::ClientNotInitialized)
    }

    pub fn client_mut(
        &mut self,
        network: Network,
    ) -> Result<&mut DirectSigningHttpRpcValidatorClient, BackendError> {
        self.signing_clients
            .get_mut(&network)
            .ok_or(BackendError::ClientNotInitialized)
    }

    pub fn current_client(&self) -> Result<&DirectSigningHttpRpcValidatorClient, BackendError> {
        self.signing_clients
            .get(&self.current_network)
            .ok_or(BackendError::ClientNotInitialized)
    }

    #[allow(unused)]
    pub fn current_client_mut(
        &mut self,
    ) -> Result<&mut DirectSigningHttpRpcValidatorClient, BackendError> {
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

    pub fn add_client(&mut self, network: Network, client: DirectSigningHttpRpcValidatorClient) {
        self.signing_clients.insert(network, client);
    }

    pub fn register_default_denoms(&mut self, network: Network) {
        let details = NymNetworkDetails::from(network);
        self.registered_coins
            .insert(network, RegisteredCoins::default_denoms(&details));
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
            .unique_by(|v| (v.nyxd_url.clone(), v.api_url.clone()));

        // Annotate with dynamic metadata
        validators.map(|v| {
            let metadata = self.validator_metadata.get(&v.nyxd_url);
            let name = v
                .nyxd_name
                .or_else(|| metadata.and_then(|m| m.name.clone()));
            config::ValidatorConfigEntry {
                nyxd_url: v.nyxd_url,
                nyxd_name: name,
                api_url: v.api_url,
            }
        })
    }

    pub fn get_nyxd_urls_only(&self, network: Network) -> impl Iterator<Item = Url> + '_ {
        self.get_config_validator_entries(network)
            .map(|v| v.nyxd_url)
    }

    pub fn get_api_urls_only(&self, network: Network) -> impl Iterator<Item = Url> + '_ {
        self.get_config_validator_entries(network)
            .filter_map(|v| v.api_url)
    }

    /// Get the list of validator nyxd urls in the network config format, suitable for passing on to
    /// the UI
    pub fn get_nyxd_urls(
        &self,
        network: Network,
    ) -> impl Iterator<Item = network_config::ValidatorUrl> + '_ {
        self.get_config_validator_entries(network)
            .map(|v| network_config::ValidatorUrl {
                url: v.nyxd_url.to_string(),
                name: v.nyxd_name,
            })
    }

    /// Get the list of nym-api urls in the network config format, suitable for passing on to
    /// the UI
    pub fn get_api_urls(
        &self,
        network: Network,
    ) -> impl Iterator<Item = network_config::ValidatorUrl> + '_ {
        self.get_config_validator_entries(network).filter_map(|v| {
            v.api_url.map(|u| network_config::ValidatorUrl {
                url: u.to_string(),
                name: None,
            })
        })
    }

    pub fn get_all_nyxd_urls(&self) -> HashMap<Network, Vec<Url>> {
        Network::iter()
            .flat_map(|network| {
                self.get_nyxd_urls_only(network)
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

    /// Fetch nyxd urls remotely. These are used to in addition to the base ones, and the user
    /// configured ones.
    pub async fn fetch_updated_nyxd_urls(&mut self) -> Result<(), BackendError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()?;
        log::debug!(
            "Fetching validator urls from: {}",
            crate::config::REMOTE_SOURCE_OF_NYXD_URLS
        );
        let response = client
            .get(crate::config::REMOTE_SOURCE_OF_NYXD_URLS.to_string())
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
        let nyxd_urls = self
            .get_all_nyxd_urls()
            .into_iter()
            .flat_map(|(_, urls)| urls.into_iter());

        // Fetch status for all urls
        let responses = fetch_status_for_urls(nyxd_urls).await?;

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

    pub async fn select_nyxd_url(
        &mut self,
        url: &str,
        network: Network,
    ) -> Result<(), BackendError> {
        if !nym_validator_client::connection_tester::test_nyxd_url_connection(
            network.into(),
            url.parse()?,
            self.config.get_mixnet_contract_address(network),
        )
        .await?
        {
            return Err(BackendError::WalletValidatorConnectionFailed);
        }
        self.config.select_nyxd_url(url.parse()?, network);
        if let Ok(client) = self.client_mut(network) {
            client.change_nyxd(url.parse()?)?;
        }
        Ok(())
    }

    pub fn reset_nyxd_url(&mut self, network: Network) -> Result<(), BackendError> {
        self.config.reset_nyxd_url(network);
        let default_nyxd = self.config.get_default_nyxd_url(network);
        if let Ok(client) = self.client_mut(network) {
            if let Some(url) = default_nyxd {
                client.change_nyxd(url)?;
            }
        }
        Ok(())
    }

    pub fn set_default_nyxd_urls(&mut self, urls: &HashMap<Network, Url>) {
        self.config.set_default_nyxd_urls(urls);
    }

    pub fn get_selected_nyxd_url(&self, network: &Network) -> Option<Url> {
        self.config.get_selected_validator_nyxd_url(*network)
    }

    pub fn get_default_nyxd_url(&self, network: &Network) -> Option<Url> {
        self.config.get_default_nyxd_url(*network)
    }

    pub fn select_nym_api_url(&mut self, url: &str, network: Network) -> Result<(), BackendError> {
        self.config.select_nym_api_url(url.parse()?, network);
        if let Ok(client) = self.client_mut(network) {
            client.change_nym_api(url.parse()?);
        }
        Ok(())
    }

    pub fn add_validator_url(&mut self, url: config::ValidatorConfigEntry, network: Network) {
        self.config.add_validator_url(url, network);
    }

    pub fn remove_validator_url(&mut self, url: config::ValidatorConfigEntry, network: Network) {
        self.config.remove_validator_url(url, network)
    }

    pub fn calculate_coin_delta(
        &self,
        coin1: &DecCoin,
        coin2: &DecCoin,
    ) -> Result<DecCoin, BackendError> {
        if coin1.denom != coin2.denom {
            return Err(BackendError::WalletPledgeUpdateInvalidCurrency);
        }

        match coin1.amount.cmp(&coin2.amount) {
            std::cmp::Ordering::Greater => {
                let delta = DecCoin {
                    amount: coin1.amount - coin2.amount,
                    denom: coin1.denom.clone(),
                };
                Ok(delta)
            }
            std::cmp::Ordering::Less => {
                let delta = DecCoin {
                    amount: coin2.amount - coin1.amount,
                    denom: coin1.denom.clone(),
                };
                Ok(delta)
            }
            std::cmp::Ordering::Equal => Ok(coin1.to_owned()),
        }
    }
}

async fn fetch_status_for_urls(
    nyxd_urls: impl Iterator<Item = Url>,
) -> Result<Vec<Result<(Url, String), reqwest::Error>>, BackendError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;

    let responses = futures::future::join_all(nyxd_urls.into_iter().map(|url| {
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
macro_rules! nyxd_client {
    ($state:ident) => {
        $state.read().await.current_client()?.nyxd
    };
}

#[macro_export]
macro_rules! api_client {
    ($state:ident) => {
        $state.read().await.current_client()?.nym_api
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_validators_urls_prepends() {
        let mut state = WalletStateInner::default();
        let _api_urls = state.get_api_urls(Network::MAINNET).collect::<Vec<_>>();

        state.add_validator_url(
            config::ValidatorConfigEntry {
                nyxd_url: "http://nyxd_url.com".parse().unwrap(),
                nyxd_name: Some("NyxdUrl".to_string()),
                api_url: Some("http://nyxd_url.com/api".parse().unwrap()),
            },
            Network::MAINNET,
        );

        state.add_validator_url(
            config::ValidatorConfigEntry {
                nyxd_url: "http://foo.com".parse().unwrap(),
                nyxd_name: None,
                api_url: None,
            },
            Network::MAINNET,
        );

        state.add_validator_url(
            config::ValidatorConfigEntry {
                nyxd_url: "http://bar.com".parse().unwrap(),
                nyxd_name: None,
                api_url: None,
            },
            Network::MAINNET,
        );

        assert_eq!(
            state
                .get_nyxd_urls_only(Network::MAINNET)
                .collect::<Vec<_>>(),
            vec![
                "http://nyxd_url.com/".parse().unwrap(),
                "http://foo.com".parse().unwrap(),
                "http://bar.com".parse().unwrap(),
                "https://rpc.nymtech.net".parse().unwrap(),
            ],
        );
        assert_eq!(
            state
                .get_api_urls_only(Network::MAINNET)
                .collect::<Vec<_>>(),
            vec![
                "http://nyxd_url.com/api".parse().unwrap(),
                "https://validator.nymtech.net/api/".parse().unwrap(),
            ],
        );
        assert_eq!(
            state
                .get_all_nyxd_urls()
                .get(&Network::MAINNET)
                .unwrap()
                .clone(),
            vec![
                "http://nyxd_url.com/".parse().unwrap(),
                "http://foo.com".parse().unwrap(),
                "http://bar.com".parse().unwrap(),
                "https://rpc.nymtech.net".parse().unwrap(),
            ],
        )
    }
}
