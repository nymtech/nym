use crate::coin::{Coin, Denom};
use crate::error::BackendError;
use crate::network::Network;
use crate::nymd_client;
use crate::state::State;

use bip39::{Language, Mnemonic};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tokio::sync::RwLock;

#[cfg_attr(test, derive(ts_rs::TS))]
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
#[derive(Serialize, Deserialize)]
pub struct CreatedAccount {
  account: Account,
  mnemonic: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
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

async fn _connect_with_mnemonic(
  mnemonic: Mnemonic,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Account, BackendError> {
  let default_network = Network::try_from(config::defaults::default_network())?;
  let mut default_account = None;
  for network in Network::iter() {
    let client = {
      let config = state.read().await.config();
      match validator_client::Client::new_signing(
        validator_client::Config::new(
          network.into(),
          config.get_nymd_validator_url(network),
          config.get_validator_api_url(network),
          config.get_mixnet_contract_address(network),
          config.get_vesting_contract_address(network),
        ),
        mnemonic.clone(),
      ) {
        Ok(client) => client,
        Err(e) => panic!("{}", e),
      }
    };

    if network == default_network {
      default_account = Some(Account::new(
        client.nymd.mixnet_contract_address()?.to_string(),
        client.nymd.address().to_string(),
        network.denom().try_into()?,
      ));
    }

    let mut w_state = state.write().await;
    w_state.add_client(network, client);
  }

  default_account
    .ok_or_else(|| BackendError::NetworkNotSupported(config::defaults::default_network()))
}
