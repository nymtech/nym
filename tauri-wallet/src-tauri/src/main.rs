#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use bip39::Mnemonic;
use error::BackendError;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;
use validator_client::nymd::{NymdClient, SigningNymdClient};

mod config;
mod error;
// mod nymd_client;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;

#[derive(Debug, Default)]
struct State {
  config: Config,
  signing_client: Option<NymdClient<SigningNymdClient>>,
}

macro_rules! format_err {
  ($e:expr) => {
    format!("line {}: {}", line!(), $e)
  };
}

#[tauri::command]
async fn connect_with_mnemonic(
  mnemonic: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<HashMap<&str, String>, String> {
  let mnemonic = match Mnemonic::from_str(&mnemonic) {
    Ok(mnemonic) => mnemonic,
    Err(e) => return Err(BackendError::from(e).to_string()),
  };
  let client;
  {
    let r_state = state.read().await;
    client = _connect_with_mnemonic(mnemonic, &r_state.config);
  }

  let mut ret = HashMap::new();
  ret.insert(
    "contract_address",
    match client.contract_address() {
      Ok(address) => address.to_string(),
      Err(e) => format_err!(e),
    },
  );
  ret.insert(
    "client_address",
    match client.contract_address() {
      Ok(address) => address.to_string(),
      Err(e) => format_err!(e),
    },
  );
  ret.insert(
    "denom",
    match client.denom() {
      Ok(denom) => denom.to_string(),
      Err(e) => format_err!(e),
    },
  );
  let mut w_state = state.write().await;
  w_state.signing_client = Some(client);
  
  Ok(ret)
}

#[tauri::command]
async fn get_balance(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<HashMap<&str, String>, String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.get_balance(client.address()).await {
      Ok(Some(coin)) => {
        let mut balance = HashMap::new();
        balance.insert("amount", coin.amount.to_string());
        balance.insert("denom", coin.denom.to_string());
        Ok(balance)
      }
      Ok(None) => Err(format!(
        "No balance available for address {}",
        client.address()
      )),
      Err(e) => Err(BackendError::from(e).to_string()),
    }
  } else {
    Err(String::from("Client has not been initialized yet"))
  }
}

fn _connect_with_mnemonic(mnemonic: Mnemonic, config: &Config) -> NymdClient<SigningNymdClient> {
  match NymdClient::connect_with_mnemonic(config.get_nymd_validator_url().unwrap(), None, mnemonic)
  {
    Ok(client) => client,
    Err(e) => panic!("{}", e),
  }
}

fn main() {
  tauri::Builder::default()
    .manage(Arc::new(RwLock::new(State::default())))
    .invoke_handler(tauri::generate_handler![connect_with_mnemonic, get_balance])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
