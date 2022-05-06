use crate::coin::{Coin, Denom};
use crate::config::Config;
use crate::error::BackendError;
use crate::network::Network as WalletNetwork;
use crate::nymd_client;
use crate::state::State;
use crate::wallet_storage::{self, StoredLogin, DEFAULT_WALLET_ACCOUNT_ID};

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
use validator_client::nymd::wallet::{AccountData, DirectSecp256k1HdWallet};

use validator_client::{nymd::SigningNymdClient, Client};

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
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/createdaccount.ts"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountEntry {
  id: String,
  address: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/balance.ts"))]
#[derive(Serialize, Deserialize)]
pub struct Balance {
  coin: Coin,
  printable_balance: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../src/types/rust/validatorurls.ts"))]
#[derive(Serialize, Deserialize)]
pub struct ValidatorUrls {
  urls: Vec<String>,
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
pub fn create_new_mnemonic() -> String {
  random_mnemonic().to_string()
}

#[tauri::command]
pub fn validate_mnemonic(mnemonic: &str) -> bool {
  Mnemonic::from_str(mnemonic).is_ok()
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

#[tauri::command]
pub async fn get_validator_nymd_urls(
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ValidatorUrls, BackendError> {
  let state = state.read().await;
  let urls: Vec<String> = state
    .get_nymd_urls(network)
    .map(|url| url.to_string())
    .collect();
  Ok(ValidatorUrls { urls })
}

#[tauri::command]
pub async fn get_validator_api_urls(
  network: WalletNetwork,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<ValidatorUrls, BackendError> {
  let state = state.read().await;
  let urls: Vec<String> = state
    .get_api_urls(network)
    .map(|url| url.to_string())
    .collect();
  Ok(ValidatorUrls { urls })
}

async fn _connect_with_mnemonic(
  mnemonic: Mnemonic,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Account, BackendError> {
  {
    let mut w_state = state.write().await;
    w_state.load_config_files();
  }

  update_validator_urls(state.clone()).await?;

  let config = {
    let state = state.read().await;

    // Take the oppertunity to list all the known validators while we have the state.
    for network in WalletNetwork::iter() {
      log::debug!(
        "List of validators for {network}: [\n{}\n]",
        state.get_validators(network).format(",\n")
      );
    }

    state.config().clone()
  };

  // Get all the urls needed for the connection test
  let (untested_nymd_urls, untested_api_urls) = {
    let state = state.read().await;
    (state.get_all_nymd_urls(), state.get_all_api_urls())
  };
  let default_nymd_urls: HashMap<WalletNetwork, Url> = untested_nymd_urls
    .iter()
    .map(|(network, urls)| (*network, urls.iter().next().unwrap().clone()))
    .collect();
  let default_api_urls: HashMap<WalletNetwork, Url> = untested_api_urls
    .iter()
    .map(|(network, urls)| (*network, urls.iter().next().unwrap().clone()))
    .collect();

  // Run connection tests on all nymd and validator-api endpoints
  let (nymd_urls, api_urls) =
    run_connection_test(untested_nymd_urls, untested_api_urls, &config).await;

  // Create clients for all networks
  let clients = create_clients(
    &nymd_urls,
    &api_urls,
    &default_nymd_urls,
    &default_api_urls,
    &config,
    &mnemonic,
  )?;

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

async fn run_connection_test(
  untested_nymd_urls: HashMap<WalletNetwork, Vec<Url>>,
  untested_api_urls: HashMap<WalletNetwork, Vec<Url>>,
  config: &Config,
) -> (
  HashMap<Network, Vec<(Url, bool)>>,
  HashMap<Network, Vec<(Url, bool)>>,
) {
  let mixnet_contract_address = WalletNetwork::iter()
    .map(|network| (network.into(), config.get_mixnet_contract_address(network)))
    .collect::<HashMap<_, _>>();

  let untested_nymd_urls = untested_nymd_urls
    .into_iter()
    .flat_map(|(net, urls)| urls.into_iter().map(move |url| (net.into(), url)));

  let untested_api_urls = untested_api_urls
    .into_iter()
    .flat_map(|(net, urls)| urls.into_iter().map(move |url| (net.into(), url)));

  validator_client::connection_tester::run_validator_connection_test(
    untested_nymd_urls,
    untested_api_urls,
    mixnet_contract_address,
  )
  .await
}

fn create_clients(
  nymd_urls: &HashMap<Network, Vec<(Url, bool)>>,
  api_urls: &HashMap<Network, Vec<(Url, bool)>>,
  default_nymd_urls: &HashMap<WalletNetwork, Url>,
  default_api_urls: &HashMap<WalletNetwork, Url>,
  config: &Config,
  mnemonic: &Mnemonic,
) -> Result<Vec<Client<SigningNymdClient>>, BackendError> {
  let mut clients = Vec::new();
  for network in WalletNetwork::iter() {
    let nymd_url = if let Some(url) = config.get_selected_validator_nymd_url(&network) {
      log::debug!("Using selected nymd_url for {network}: {url}");
      url.clone()
    } else {
      let default_nymd_url = default_nymd_urls
        .get(&network)
        .expect("Expected at least one nymd_url");
      select_random_responding_url(nymd_urls, network).unwrap_or_else(|| {
        log::debug!("No successful nymd_urls for {network}: using default: {default_nymd_url}");
        default_nymd_url.clone()
      })
    };

    let api_url = if let Some(url) = config.get_selected_validator_api_url(&network) {
      log::debug!("Using selected api_url for {network}: {url}");
      url.clone()
    } else {
      let default_api_url = default_api_urls
        .get(&network)
        .expect("Expected at least one api url");
      select_first_responding_url(api_urls, network).unwrap_or_else(|| {
        log::debug!("No passing api_urls for {network}: using default: {default_api_url}");
        default_api_url.clone()
      })
    };

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

fn select_random_responding_url(
  urls: &HashMap<Network, Vec<(Url, bool)>>,
  network: WalletNetwork,
) -> Option<Url> {
  urls.get(&network.into()).and_then(|urls| {
    let urls: Vec<_> = urls
      .iter()
      .filter_map(|(url, result)| if *result { Some(url.clone()) } else { None })
      .collect();
    urls.choose(&mut rand::thread_rng()).cloned()
  })
}

fn select_first_responding_url(
  urls: &HashMap<Network, Vec<(Url, bool)>>,
  network: WalletNetwork,
  //config: &Config,
) -> Option<Url> {
  urls.get(&network.into()).and_then(|urls| {
    urls
      .iter()
      .find_map(|(url, result)| if *result { Some(url.clone()) } else { None })
  })
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
pub fn create_password(mnemonic: &str, password: String) -> Result<(), BackendError> {
  if does_password_file_exist()? {
    return Err(BackendError::WalletFileAlreadyExists);
  }
  log::info!("Creating password");

  let mnemonic = Mnemonic::from_str(mnemonic)?;
  let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
  // Currently we only support a single, default, id in the wallet
  let id = wallet_storage::AccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
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
  let id = wallet_storage::AccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
  let password = wallet_storage::UserPassword::new(password);
  let stored_account = wallet_storage::load_existing_wallet_login_information(&id, &password)?;
  let (mnemonic, all_accounts) = extract_mnemonic_and_all_accounts(stored_account, id)?;

  {
    let mut w_state = state.write().await;
    w_state.set_all_accounts(all_accounts);
  }

  _connect_with_mnemonic(mnemonic, state).await
}

fn extract_mnemonic_and_all_accounts(
  stored_account: StoredLogin,
  id: wallet_storage::AccountId,
) -> Result<(Mnemonic, Vec<wallet_storage::WalletAccount>), BackendError> {
  let mnemonic = match stored_account {
    StoredLogin::Mnemonic(ref account) => account.mnemonic().clone(),
    StoredLogin::Multiple(ref accounts) => {
      // Login using the first account in the list
      accounts
        .get_accounts()
        .next()
        .ok_or(BackendError::NoSuchIdInWalletLoginEntry)?
        .account
        .mnemonic()
        .clone()
    }
  };

  // Keep track of all accounts for that id
  let all_accounts: Vec<_> = stored_account
    .unwrap_into_multiple_accounts(id)
    .into_accounts()
    .collect();

  Ok((mnemonic, all_accounts))
}

#[tauri::command]
pub fn remove_password() -> Result<(), BackendError> {
  log::info!("Removing password");
  let id = wallet_storage::AccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
  wallet_storage::remove_wallet_login_information(&id)
}

#[tauri::command]
pub async fn add_account_for_password(
  mnemonic: &str,
  password: &str,
  inner_id: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<AccountEntry, BackendError> {
  log::info!("Adding account for the current password: {inner_id}");
  let mnemonic = Mnemonic::from_str(mnemonic)?;
  let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();
  // Currently we only support a single, default, id in the wallet
  let id = wallet_storage::AccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
  let inner_id = wallet_storage::AccountId::new(inner_id.to_string());
  let password = wallet_storage::UserPassword::new(password.to_string());

  // Creating the returned account entry could fail, so do it before attempting to store to wallet
  let address = {
    let state = state.read().await;
    let network: Network = state.current_network().into();
    derive_address(mnemonic.clone(), network.bech32_prefix())?.to_string()
  };

  wallet_storage::append_account_to_wallet_login_information(
    mnemonic,
    hd_path,
    id.clone(),
    inner_id.clone(),
    &password,
  )?;

  // Re-read all the acccounts from the  wallet to reset the state, rather than updating it
  // incrementally
  reset_state_with_all_accounts_from_file(&id, &password, state).await?;

  Ok(AccountEntry {
    id: inner_id.to_string(),
    address,
  })
}

async fn reset_state_with_all_accounts_from_file(
  id: &wallet_storage::AccountId,
  password: &wallet_storage::UserPassword,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  let stored_account = wallet_storage::load_existing_wallet_login_information(id, password)?;
  let all_accounts: Vec<_> = stored_account
    .unwrap_into_multiple_accounts(id.clone())
    .into_accounts()
    .collect();

  let mut w_state = state.write().await;
  w_state.set_all_accounts(all_accounts);
  Ok(())
}

#[tauri::command]
pub async fn remove_account_for_password(
  password: &str,
  inner_id: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), BackendError> {
  log::info!("Removing account: {inner_id}");
  // Currently we only support a single, default, id in the wallet
  let id = wallet_storage::AccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
  let inner_id = wallet_storage::AccountId::new(inner_id.to_string());
  let password = wallet_storage::UserPassword::new(password.to_string());
  wallet_storage::remove_account_from_wallet_login(&id, &inner_id, &password)?;

  reset_state_with_all_accounts_from_file(&id, &password, state).await
}

fn derive_address(
  mnemonic: bip39::Mnemonic,
  prefix: &str,
) -> Result<cosmrs::AccountId, BackendError> {
  DirectSecp256k1HdWallet::from_mnemonic(prefix, mnemonic)?
    .try_derive_accounts()?
    .first()
    .map(AccountData::address)
    .cloned()
    .ok_or(BackendError::FailedToDeriveAddress)
}

#[tauri::command]
pub async fn list_accounts(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Vec<AccountEntry>, BackendError> {
  let state = state.read().await;
  let network: Network = state.current_network().into();
  let prefix = network.bech32_prefix();

  let all_accounts = state
    .get_all_accounts()
    .map(|account| AccountEntry {
      id: account.id.to_string(),
      address: derive_address(account.account.mnemonic().clone(), prefix)
        .unwrap()
        .to_string(),
    })
    .collect();

  Ok(all_accounts)
}

#[tauri::command]
pub fn show_mnemonic_for_account_in_password(
  account_id: String,
  password: String,
) -> Result<String, BackendError> {
  log::info!("Getting mnemonic for: {account_id}");
  let id = wallet_storage::AccountId::new(DEFAULT_WALLET_ACCOUNT_ID.to_string());
  let account_id = wallet_storage::AccountId::new(account_id);
  let password = wallet_storage::UserPassword::new(password);
  let stored_account = wallet_storage::load_existing_wallet_login_information(&id, &password)?;

  let mnemonic = match stored_account {
    StoredLogin::Mnemonic(_) => return Err(BackendError::WalletUnexpectedMnemonicAccount),
    StoredLogin::Multiple(ref accounts) => accounts
      .get_account(&account_id)
      .ok_or(BackendError::NoSuchIdInWalletLoginEntry)?
      .account
      .mnemonic()
      .clone(),
  };

  Ok(mnemonic.to_string())
}

#[tauri::command]
pub async fn sign_in_decrypted_account(
  account_id: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Account, BackendError> {
  log::info!("Signing in to already decrypted account: {account_id}");
  let mnemonic = {
    let state = state.read().await;
    let account = &state
      .get_all_accounts()
      .find(|a| a.id.as_ref() == account_id)
      .ok_or(BackendError::NoSuchIdInWalletLoginEntry)?
      .account;
    account.mnemonic().clone()
  };
  _connect_with_mnemonic(mnemonic, state).await
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::path::PathBuf;

  use crate::wallet_storage;

  // This decryptes a stored wallet file using the same procedure as when signing in. Most tests
  // related to the encryped wallet storage is in `wallet_storage`.
  #[test]
  fn decrypt_stored_wallet_for_sign_in() {
    const SAVED_WALLET: &str = "src/wallet_storage/test-data/saved-wallet.json";
    let wallet_file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAVED_WALLET);
    let id = wallet_storage::AccountId::new("first".to_string());
    let password = wallet_storage::UserPassword::new("password".to_string());
    let hd_path: DerivationPath = COSMOS_DERIVATION_PATH.parse().unwrap();

    let stored_account =
      wallet_storage::load_existing_wallet_login_information_at_file(wallet_file, &id, &password)
        .unwrap();
    let (mnemonic, all_accounts) =
      extract_mnemonic_and_all_accounts(stored_account, id.clone()).unwrap();

    let expected_mnemonic = bip39::Mnemonic::from_str("country mean universe text phone begin deputy reject result good cram illness common cluster proud swamp digital patrol spread bar face december base kick").unwrap();
    assert_eq!(mnemonic, expected_mnemonic);

    assert_eq!(
      all_accounts,
      vec![wallet_storage::WalletAccount::new_mnemonic_backed_account(
        id,
        expected_mnemonic,
        hd_path,
      )]
    );
  }
}
