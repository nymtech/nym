use crate::coin::{Coin, Denom};
use crate::config::{Config, ValidatorWithApiEndpoint};
use crate::error::BackendError;
use crate::network::Network;
use crate::nymd_client;
use crate::state::State;

use bip39::{Language, Mnemonic};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use strum::IntoEnumIterator;
use tokio::sync::RwLock;
use validator_client::nymd::error::NymdError;
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
  let validators = choose_validators(mnemonic.clone(), &state).await?;

  let config = state.read().await.config();
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
  validators: &HashMap<Network, ValidatorWithApiEndpoint>,
  mnemonic: &Mnemonic,
  config: &Config,
) -> Result<Vec<Client<SigningNymdClient>>, BackendError> {
  let mut clients = Vec::new();
  for network in Network::iter() {
    let client = validator_client::Client::new_signing(
      validator_client::Config::new(
        network.into(),
        validators[&network].nymd_url.clone(),
        validators[&network].api_url.clone(),
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

async fn choose_validators(
  mnemonic: Mnemonic,
  state: &tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<HashMap<Network, ValidatorWithApiEndpoint>, BackendError> {
  let config = state.read().await.config();

  // Try to connect to validators on all networks
  let mut validators = select_responding_validators(&config, &mnemonic).await?;

  // If for a network we didn't manage to connect to any validators, just go ahead and try with the
  // first in the list
  for network in Network::iter() {
    validators.entry(network).or_insert_with(|| {
      let default_validator = config
        .get_validators_with_api_endpoint(network)
        .next()
        // We always have at least one hardcoded default validator
        .unwrap();
      println!(
        "Using default for {network}: {}, {}",
        default_validator.nymd_url, default_validator.api_url,
      );
      default_validator
    });
  }
  Ok(validators)
}

// For each network, try the list of available validators one by one and use the first responding
// one.
async fn select_responding_validators(
  config: &Config,
  mnemonic: &Mnemonic,
) -> Result<HashMap<Network, ValidatorWithApiEndpoint>, BackendError> {
  use tokio::time::timeout;
  let validators = futures::future::join_all(Network::iter().map(|network| {
    timeout(
      Duration::from_millis(3000),
      try_connect_to_validators(
        config.get_validators_with_api_endpoint(network),
        config,
        network,
        mnemonic.clone(),
      ),
    )
  }))
  .await;

  // Drop networks that failed the global timeout
  let validators = validators.into_iter().filter_map(Result::ok);

  // Rewrap to return any errors during client creation
  let validators = validators.collect::<Result<Vec<_>, _>>()?;

  // Filter out networks where we exhausted all listed validators
  let validators = validators.into_iter().flatten();

  Ok(validators.collect::<HashMap<_, _>>())
}

async fn try_connect_to_validators(
  validators: impl Iterator<Item = ValidatorWithApiEndpoint>,
  config: &Config,
  network: Network,
  mnemonic: Mnemonic,
) -> Result<Option<(Network, ValidatorWithApiEndpoint)>, BackendError> {
  for validator in validators {
    if let Some(responding_validator) =
      try_connect_to_validator(&validator, config, network, mnemonic.clone()).await?
    {
      // Pick the first successful one
      return Ok(Some(responding_validator));
    }
  }
  Ok(None)
}

async fn try_connect_to_validator(
  validator: &ValidatorWithApiEndpoint,
  config: &Config,
  network: Network,
  mnemonic: Mnemonic,
) -> Result<Option<(Network, ValidatorWithApiEndpoint)>, BackendError> {
  let client = validator_client::Client::new_signing(
    validator_client::Config::new(
      network.into(),
      validator.nymd_url.clone(),
      validator.api_url.clone(),
      config.get_mixnet_contract_address(network),
      config.get_vesting_contract_address(network),
      config.get_bandwidth_claim_contract_address(network),
    ),
    mnemonic,
  )?;

  if is_validator_connection_ok(&client).await {
    println!(
      "Connection ok for {network}: {}, {}",
      validator.nymd_url, validator.api_url
    );
    Ok(Some((network, validator.clone())))
  } else {
    Ok(None)
  }
}

// The criteria used to determina if a validator endpoint is to be used
async fn is_validator_connection_ok(client: &Client<SigningNymdClient>) -> bool {
  match client.get_mixnet_contract_version().await {
    Err(NymdError::TendermintError(_)) => false,
    Err(_) | Ok(_) => true,
  }
}
