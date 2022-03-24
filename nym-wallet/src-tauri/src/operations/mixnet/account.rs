use crate::coin::{Coin, Denom};
use crate::config::{Config, ValidatorUrl};
use crate::error::BackendError;
use crate::network::Network;
use crate::nymd_client;
use crate::state::State;

use bip39::{Language, Mnemonic};
use rand::seq::SliceRandom;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tokio::sync::RwLock;
use url::Url;
use validator_client::nymd::SigningNymdClient;
use validator_client::Client;

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/account.ts"))]
#[derive(Serialize, Deserialize)]
pub struct Account {
  contract_address: String,
  client_address: String,
  denom: Denom,
}

impl Account {
  pub fn new(contract_address: String, client_address: String, denom: Denom) -> Self {
    Account {
      contract_address,
      client_address,
      denom,
    }
  }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/createdaccount.ts"))]
#[derive(Serialize, Deserialize)]
pub struct CreatedAccount {
  account: Account,
  mnemonic: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/balance.ts"))]
#[derive(Serialize, Deserialize)]
pub struct Balance {
  coin: Coin,
  printable_balance: String,
}

#[tauri::command]
pub async fn connect_with_mnemonic(
  mnemonic: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Account, BackendError> {
  let mnemonic = Mnemonic::from_str(&mnemonic)?;
  _connect_with_mnemonic(mnemonic, state).await
}

#[tauri::command]
pub async fn get_balance(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Balance, BackendError> {
  let denom = state.read().await.current_network().denom();
  match nymd_client!(state)
    .get_balance(nymd_client!(state).address(), denom.clone())
    .await
  {
    Ok(Some(coin)) => {
      let coin = Coin::new(
        &coin.amount.to_string(),
        &Denom::from_str(&coin.denom.to_string())?,
      );
      Ok(Balance {
        coin: coin.clone(),
        printable_balance: format!("{} {}", coin.to_major().amount(), &denom.as_ref()[1..]),
      })
    }
    Ok(None) => Err(BackendError::NoBalance(
      nymd_client!(state).address().to_string(),
    )),
    Err(e) => Err(BackendError::from(e)),
  }
}

#[tauri::command]
pub async fn create_new_account(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<CreatedAccount, BackendError> {
  let rand_mnemonic = random_mnemonic();
  let account = connect_with_mnemonic(rand_mnemonic.to_string(), state).await?;
  Ok(CreatedAccount {
    account,
    mnemonic: rand_mnemonic.to_string(),
  })
}

#[tauri::command]
pub fn create_new_mnemonic() -> Result<String, BackendError> {
  let rand_mnemonic = random_mnemonic();
  Ok(rand_mnemonic.to_string())
}

#[tauri::command]
pub async fn switch_network(
  state: tauri::State<'_, Arc<RwLock<State>>>,
  network: Network,
) -> Result<Account, BackendError> {
  let account = {
    let r_state = state.read().await;
    let client = r_state.client(network)?;
    let denom = network.denom();

    Account::new(
      client.nymd.mixnet_contract_address()?.to_string(),
      client.nymd.address().to_string(),
      denom.try_into()?,
    )
  };

  let mut w_state = state.write().await;
  w_state.set_network(network);

  Ok(account)
}

#[tauri::command]
pub async fn logout(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), BackendError> {
  state.write().await.logout();
  Ok(())
}

fn random_mnemonic() -> Mnemonic {
  let mut rng = rand::thread_rng();
  Mnemonic::generate_in_with(&mut rng, Language::English, 24).unwrap()
}

#[tauri::command]
pub async fn update_validator_urls(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  // Update the list of validators by fecthing additional ones remotely. If it fails, just ignore.
  let mut w_state = state.write().await;
  let _r = w_state.fetch_updated_validator_urls().await;
  Ok(())
}

async fn _connect_with_mnemonic(
  mnemonic: Mnemonic,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Account, BackendError> {
  update_validator_urls(state.clone()).await?;

  let config = state.read().await.config().clone();
  let validators = config.check_validator_health_for_all_networks().await?;
  let clients = create_clients(&validators, &mnemonic, &config)?;

  // Set the default account
  let default_network: Network = config::defaults::DEFAULT_NETWORK.into();
  let client_for_default_network = clients
    .iter()
    .find(|client| Network::from(client.network) == default_network);
  let account_for_default_network = match client_for_default_network {
    Some(client) => Ok(Account::new(
      client.nymd.mixnet_contract_address()?.to_string(),
      client.nymd.address().to_string(),
      default_network.denom().try_into()?,
    )),
    None => Err(BackendError::NetworkNotSupported(
      config::defaults::DEFAULT_NETWORK,
    )),
  };

  // Register all the clients
  for client in clients {
    let network: Network = client.network.into();
    let mut w_state = state.write().await;
    w_state.add_client(network, client);
  }

  account_for_default_network
}

fn create_clients(
  validators: &HashMap<Network, Vec<(ValidatorUrl, StatusCode)>>,
  mnemonic: &Mnemonic,
  config: &Config,
) -> Result<Vec<Client<SigningNymdClient>>, BackendError> {
  let mut clients = Vec::new();
  for network in Network::iter() {
    let nymd_url = select_validator_nymd_url(network, validators)?;
    let api_url = select_validator_api_url(network, validators)?;
    log::info!("{network}: nymd_url: connecting to {nymd_url}");
    log::info!("{network}: api_url: connecting to {api_url}");

    let client = validator_client::Client::new_signing(
      validator_client::Config::new(
        network.into(),
        nymd_url,
        api_url,
        config.get_mixnet_contract_address(network),
        config.get_vesting_contract_address(network),
        config.get_bandwidth_claim_contract_address(network),
      ),
      mnemonic.clone(),
    )?;
    clients.push(client);
  }
  Ok(clients)
}

fn select_validator_nymd_url(
  network: Network,
  validators: &HashMap<Network, Vec<(ValidatorUrl, StatusCode)>>,
) -> Result<Url, BackendError> {
  // For the nymd url we pick one at random
  validators[&network]
    .choose(&mut rand::thread_rng())
    .map(|v| v.0.nymd_url.clone())
    .ok_or(BackendError::NoNymdValidatorConfigured)
}

fn select_validator_api_url(
  network: Network,
  validators: &HashMap<Network, Vec<(ValidatorUrl, StatusCode)>>,
) -> Result<Url, BackendError> {
  // For the validator api we always just pick the first one
  validators[&network]
    .get(0)
    .and_then(|v| v.0.api_url.clone())
    .ok_or(BackendError::NoValidatorApiUrlConfigured)
}
