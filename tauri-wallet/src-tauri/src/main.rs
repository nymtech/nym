#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use bip39::Mnemonic;
use cosmos_sdk::{AccountId, Coin, Decimal};
use error::BackendError;
use std::collections::HashMap;
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

fn printable_coin(coin: Option<Coin>) -> Result<String, String> {
  if let Some(coin) = coin {
    let amount = match native_to_printable(&coin.amount.to_string()) {
      Ok(amount) => amount,
      Err(e) => return Err(e),
    };
    let ticker = if coin.denom.to_string().starts_with("u") {
      coin.denom.to_string()[1..].to_uppercase()
    } else {
      coin.denom.to_string().to_uppercase()
    };
    Ok(format!("{} {}", amount, ticker))
  } else {
    Ok("0".to_string())
  }
}

fn printable_balance(balance: Option<Vec<Coin>>) -> Result<String, String> {
  if let Some(balance) = balance {
    if !balance.is_empty() {
      return Ok(
        balance
          .into_iter()
          .map(|coin| match printable_coin(Some(coin)) {
            Ok(native) => native,
            Err(e) => e,
          })
          .collect::<Vec<String>>()
          .join(", "),
      );
    }
  }
  Ok("-".to_string())
}

// converts display amount, such as "12.0346" to its native token representation,
// with 6 fractional digits. So in that case it would result in "12034600"
// Basically does the same job as `displayAmountToNative` but without the requirement
// of having the coinMap
#[tauri::command]
fn printable_balance_to_native(amount: &str) -> Result<String, String> {
  match amount.parse::<f64>() {
    Ok(f) => match Decimal::from_str(&(f * 1_000_000.).to_string()) {
      Ok(amount) => Ok(amount.to_string()),
      Err(e) => Err(format_err!(format!(
        "Could not convert `{}` to Decimal",
        amount
      ))),
    },
    Err(e) => Err(format_err!(format!(
      "Could not convert `{}` to f64",
      amount
    ))),
  }
}

#[tauri::command]
fn native_to_printable(native_value: &str) -> Result<String, String> {
  match Decimal::from_str(native_value) {
    Ok(decimal) => Ok(format!(
      "{}",
      decimal.to_string().parse::<f64>().unwrap() / 1_000_000.
    )),
    Err(e) => Err(format_err!(format!(
      "Could not convert `{}` to Decimal",
      native_value
    ))),
  }
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
  ret.insert("client_address", client.address().to_string());
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
        balance.insert("printableBalance", printable_coin(Some(coin))?);
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
  match NymdClient::connect_with_mnemonic(
    config.get_nymd_validator_url().unwrap(),
    Some(AccountId::from_str(&config.get_mixnet_contract_address()).unwrap()),
    mnemonic,
  ) {
    Ok(client) => client,
    Err(e) => panic!("{}", e),
  }
}

fn main() {
  tauri::Builder::default()
    .manage(Arc::new(RwLock::new(State::default())))
    .invoke_handler(tauri::generate_handler![
      connect_with_mnemonic,
      get_balance,
      printable_balance_to_native,
      native_to_printable
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
