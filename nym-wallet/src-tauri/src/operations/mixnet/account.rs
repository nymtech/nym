use crate::coin::{Coin, Denom};
use crate::config::Config;
use crate::error::BackendError;
use crate::network::Network as WalletNetwork;
use crate::nymd_client;
use crate::state::State;
use crate::wallet_storage::{self, DEFAULT_WALLET_ACCOUNT_ID};

use bip39::{Language, Mnemonic};
use config::defaults::all::Network;
use config::defaults::COSMOS_DERIVATION_PATH;
use cosmrs::bip32::DerivationPath;
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tokio::sync::RwLock;
use url::Url;

use validator_client::{
  connection_tester::run_validator_connection_test, nymd::SigningNymdClient, Client,
};

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
pub async fn create_new_mnemonic() -> Result<String, BackendError> {
  let rand_mnemonic = random_mnemonic();
  Ok(rand_mnemonic.to_string())
}

#[tauri::command]
pub async fn switch_network(
  state: tauri::State<'_, Arc<RwLock<State>>>,
  network: WalletNetwork,
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
  let config = state.read().await.config();

  for network in WalletNetwork::iter() {
    log::debug!(
      "List of validators for {network}: [\n{}\n]",
      config.get_validators(network).format(",\n")
    );
  }

  // Run connection tests on all nymd and validator-api endpoints
  let (nymd_urls, api_urls) = {
    let mixnet_contract_address = WalletNetwork::iter()
      .map(|network| (network.into(), config.get_mixnet_contract_address(network)))
      .collect::<HashMap<_, _>>();
    let nymd_urls = WalletNetwork::iter().flat_map(|network| {
      config
        .get_nymd_urls(network)
        .map(move |url| (network.into(), url))
    });
    let api_urls = WalletNetwork::iter().flat_map(|network| {
      config
        .get_api_urls(network)
        .map(move |url| (network.into(), url))
    });

    run_validator_connection_test(nymd_urls, api_urls, mixnet_contract_address).await
  };

  let clients = create_clients(&nymd_urls, &api_urls, &mnemonic, &config)?;

  // Set the default account
  let default_network: WalletNetwork = config::defaults::DEFAULT_NETWORK.into();
  let client_for_default_network = clients
    .iter()
    .find(|client| WalletNetwork::from(client.network) == default_network);
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
    let network: WalletNetwork = client.network.into();
    let mut w_state = state.write().await;
    w_state.add_client(network, client);
  }

  account_for_default_network
}

fn select_random_responding_nymd_url(
  nymd_urls: &HashMap<Network, Vec<(Url, bool)>>,
  network: WalletNetwork,
  config: &Config,
) -> Url {
  // We pick a randon responding nymd url, and if not, fall back on the first one in the list.
  nymd_urls
    .get(&network.into())
    .and_then(|urls| {
      let nymd_urls: Vec<_> = urls
        .iter()
        .filter_map(|(url, result)| if *result { Some(url.clone()) } else { None })
        .collect();
      nymd_urls.choose(&mut rand::thread_rng()).cloned()
    })
    .unwrap_or_else(|| {
      log::debug!("No passing nymd_urls for {network}: using default");
      config
        .get_nymd_urls(network)
        .next()
        .expect("Expected at least one hardcoded nymd url")
    })
}

fn select_first_responding_api_url(
  api_urls: &HashMap<Network, Vec<(Url, bool)>>,
  network: WalletNetwork,
  config: &Config,
) -> Url {
  // We pick the first API url among the responding ones. If none exists, fall back on the first
  // one in the list.
  api_urls
    .get(&network.into())
    .and_then(|urls| {
      urls
        .iter()
        .find_map(|(url, result)| if *result { Some(url.clone()) } else { None })
    })
    .unwrap_or_else(|| {
      log::debug!("No passing api_urls for {network}: using default");
      config
        .get_api_urls(network)
        .next()
        .expect("Expected at least one hardcoded api url")
    })
}

fn create_clients(
  nymd_urls: &HashMap<Network, Vec<(Url, bool)>>,
  api_urls: &HashMap<Network, Vec<(Url, bool)>>,
  mnemonic: &Mnemonic,
  config: &Config,
) -> Result<Vec<Client<SigningNymdClient>>, BackendError> {
  let mut clients = Vec::new();
  for network in WalletNetwork::iter() {
    let nymd_url = select_random_responding_nymd_url(nymd_urls, network, config);
    let api_url = select_first_responding_api_url(api_urls, network, config);

    log::info!("Connecting to: nymd_url: {nymd_url} for {network}");
    log::info!("Connecting to: api_url: {api_url} for {network}");

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

#[tauri::command]
pub fn does_password_file_exist() -> Result<bool, BackendError> {
  log::info!("Checking wallet file");
  let file = wallet_storage::wallet_login_filepath()?;
  if file.exists() {
    log::info!("Exists: {}", file.to_string_lossy());
    Ok(true)
  } else {
    log::info!("Does not exist: {}", file.to_string_lossy());
    Ok(false)
  }
}

#[tauri::command]
pub fn create_password(mnemonic: String, password: String) -> Result<(), BackendError> {
  if does_password_file_exist()? {
    return Err(BackendError::WalletFileAlreadyExists);
  }
  log::info!("Creating password");

  let mnemonic = Mnemonic::from_str(&mnemonic)?;
  let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
  // Currently we only support a single, default, id in the wallet
  let id = wallet_storage::WalletAccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
  let password = wallet_storage::UserPassword::new(password);
  wallet_storage::store_wallet_login_information(mnemonic, hd_path, id, &password)
}

#[tauri::command]
pub async fn sign_in_with_password(
  password: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Account, BackendError> {
  log::info!("Signing in with password");

  // Currently we only support a single, default, id in the wallet
  let id = wallet_storage::WalletAccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
  let password = wallet_storage::UserPassword::new(password);
  let stored_account = wallet_storage::load_existing_wallet_login_information(&id, &password)?;
  _connect_with_mnemonic(stored_account.mnemonic().clone(), state).await
}
