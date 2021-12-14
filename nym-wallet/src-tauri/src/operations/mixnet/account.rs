use crate::client;
use crate::coin::{Coin, Denom};
use crate::config::Config;
use crate::error::BackendError;
use crate::state::State;
use bip39::{Language, Mnemonic};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::{AccountId, NymdClient, SigningNymdClient};

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize)]
pub struct Account {
  contract_address: String,
  client_address: String,
  denom: Denom,
  mnemonic: Option<String>,
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
  let client = {
    let r_state = state.read().await;
    _connect_with_mnemonic(mnemonic, &r_state.config())
  };

  let contract_address = client.mixnet_contract_address()?.to_string();
  let client_address = client.address().to_string();
  let denom = client.denom()?;

  let account = Account {
    contract_address,
    client_address,
    denom: Denom::from_str(&denom.to_string())?,
    mnemonic: None,
  };

  let mut w_state = state.write().await;
  w_state.set_client(client);

  Ok(account)
}

#[tauri::command]
pub async fn get_balance(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Balance, BackendError> {
  match client!(state).get_balance(client!(state).address()).await {
    Ok(Some(coin)) => {
      let coin = Coin::new(
        &coin.amount.to_string(),
        &Denom::from_str(&coin.denom.to_string())?,
      );
      Ok(Balance {
        coin: coin.clone(),
        printable_balance: coin.to_major().to_string(),
      })
    }
    Ok(None) => Err(BackendError::NoBalance(
      client!(state).address().to_string(),
    )),
    Err(e) => Err(BackendError::from(e)),
  }
}

#[tauri::command]
pub async fn create_new_account(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Account, BackendError> {
  let rand_mnemonic = random_mnemonic();
  let mut client = connect_with_mnemonic(rand_mnemonic.to_string(), state).await?;
  client.mnemonic = Some(rand_mnemonic.to_string());
  Ok(client)
}

fn random_mnemonic() -> Mnemonic {
  let mut rng = rand::thread_rng();
  Mnemonic::generate_in_with(&mut rng, Language::English, 24).unwrap()
}

fn _connect_with_mnemonic(mnemonic: Mnemonic, config: &Config) -> NymdClient<SigningNymdClient> {
  match NymdClient::connect_with_mnemonic(
    config.get_nymd_validator_url().unwrap(),
    Some(AccountId::from_str(&config.get_mixnet_contract_address()).unwrap()),
    None,
    mnemonic,
    None,
  ) {
    Ok(client) => client,
    Err(e) => panic!("{}", e),
  }
}
