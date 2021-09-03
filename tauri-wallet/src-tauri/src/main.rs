#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use ::config::defaults::DENOM;
use bip39::Mnemonic;
use cosmos_sdk::Coin as CosmosCoin;
use cosmos_sdk::Denom as CosmosDenom;
use cosmos_sdk::{AccountId, Decimal};
use cosmwasm_std::Coin as CosmWasmCoin;
use error::BackendError;
use mixnet_contract::{Gateway, MixNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::str::FromStr;
use ts_rs::{export, TS};
use validator_client::nymd::{NymdClient, SigningNymdClient};

mod config;
mod error;
// mod nymd_client;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;

macro_rules! format_err {
  ($e:expr) => {
    format!("line {}: {}", line!(), $e)
  };
}

#[derive(TS, Serialize, Deserialize)]
struct Balance {
  coin: Coin,
  printable_balance: String,
}

enum Denom {
  Major,
  Minor,
}

impl fmt::Display for Denom {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *self {
      Denom::Major => f.write_str(&DENOM[1..].to_uppercase()),
      Denom::Minor => f.write_str(DENOM),
    }
  }
}

impl FromStr for Denom {
  type Err = String;

  fn from_str(s: &str) -> Result<Denom, String> {
    if s.to_lowercase() == DENOM.to_lowercase() {
      Ok(Denom::Minor)
    } else if s.to_lowercase() == DENOM[1..].to_lowercase() {
      Ok(Denom::Major)
    } else {
      Err(format_err!(format!(
        "{} is not a valid denomination string",
        s
      )))
    }
  }
}

// Proxy types to allow TS generation
#[derive(TS, Serialize, Deserialize, Clone)]
struct Coin {
  amount: String,
  denom: String,
}

impl fmt::Display for Coin {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&format!("{} {}", self.amount, self.denom))
  }
}

impl Coin {
  fn to_major(&self) -> Coin {
    if let Ok(denom) = Denom::from_str(&self.denom) {
      match denom {
        Denom::Major => self.clone(),
        Denom::Minor => Coin {
          amount: (self.amount.parse::<f64>().unwrap() / 1_000_000.).to_string(),
          denom: Denom::Major.to_string(),
        },
      }
    } else {
      unreachable!()
    }
  }

  fn to_minor(&self) -> Coin {
    if let Ok(denom) = Denom::from_str(&self.denom) {
      match denom {
        Denom::Minor => self.clone(),
        Denom::Major => Coin {
          amount: (self.amount.parse::<f64>().unwrap() * 1_000_000.).to_string(),
          denom: Denom::Minor.to_string(),
        },
      }
    } else {
      unreachable!()
    }
  }
}

impl TryFrom<Coin> for CosmWasmCoin {
  type Error = String;

  fn try_from(coin: Coin) -> Result<CosmWasmCoin, String> {
    match serde_json::to_value(coin) {
      Ok(value) => match serde_json::from_value(value) {
        Ok(coin) => Ok(coin),
        Err(e) => Err(format_err!(e)),
      },
      Err(e) => Err(format_err!(e)),
    }
  }
}

// There is some confusion here over coins and denoms, it feels like we should use types to differentiate between the two
impl TryFrom<Coin> for CosmosCoin {
  type Error = String;

  fn try_from(coin: Coin) -> Result<CosmosCoin, String> {
    let coin = coin.to_minor();
    match Decimal::from_str(&coin.amount) {
      Ok(d) => Ok(CosmosCoin {
        amount: d,
        denom: CosmosDenom::from_str(&coin.denom).unwrap(),
      }),
      Err(e) => Err(format_err!(e)),
    }
  }
}

impl From<CosmosCoin> for Coin {
  fn from(c: CosmosCoin) -> Coin {
    Coin {
      amount: c.amount.to_string(),
      denom: c.denom.to_string(),
    }
  }
}

#[derive(Debug, Default)]
struct State {
  config: Config,
  signing_client: Option<NymdClient<SigningNymdClient>>,
}

#[tauri::command]
fn major_to_minor(amount: String) -> Result<Coin, String> {
  let coin = Coin {
    amount,
    denom: Denom::Major.to_string(),
  };
  Ok(coin.to_minor())
}

#[tauri::command]
fn minor_to_major(amount: String) -> Result<Coin, String> {
  let coin = Coin {
    amount,
    denom: Denom::Minor.to_string(),
  };
  Ok(coin.to_major())
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
async fn get_balance(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<Balance, String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.get_balance(client.address()).await {
      Ok(Some(coin)) => {
        let coin = Coin {
          amount: coin.amount.to_string(),
          denom: coin.denom.to_string(),
        };
        Ok(Balance {
          coin: coin.clone(),
          printable_balance: coin.to_major().to_string(),
        })
      }
      Ok(None) => Err(format!(
        "No balance available for address {}",
        client.address()
      )),
      Err(e) => Err(BackendError::from(e).to_string()),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn owns_mixnode(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<bool, String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.owns_mixnode(client.address()).await {
      Ok(o) => Ok(o),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn owns_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<bool, String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.owns_gateway(client.address()).await {
      Ok(o) => Ok(o),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn unbond_mixnode(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.unbond_mixnode().await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn bond_mixnode(
  mixnode: MixNode,
  bond: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  let bond: CosmWasmCoin = match bond.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  if let Some(client) = &r_state.signing_client {
    match client.bond_mixnode(mixnode, bond).await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn delegate_to_mixnode(
  identity: String,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  let bond: CosmWasmCoin = match amount.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  if let Some(client) = &r_state.signing_client {
    match client.delegate_to_mixnode(identity, bond).await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn undelegate_from_mixnode(
  identity: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.remove_mixnode_delegation(identity).await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn delegate_to_gateway(
  identity: String,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  let bond: CosmWasmCoin = match amount.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  if let Some(client) = &r_state.signing_client {
    match client.delegate_to_gateway(identity, bond).await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn undelegate_from_gateway(
  identity: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.remove_gateway_delegation(identity).await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn bond_gateway(
  gateway: Gateway,
  bond: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<(), String> {
  let r_state = state.read().await;
  let bond: CosmWasmCoin = match bond.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  if let Some(client) = &r_state.signing_client {
    match client.bond_gateway(gateway, bond).await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn unbond_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), String> {
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.unbond_gateway().await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
  }
}

#[tauri::command]
async fn send(address: &str, amount: Coin, memo: String, state: tauri::State<'_, Arc<RwLock<State>>>,) -> Result<(), String> {
  let address = match AccountId::from_str(address) {
    Ok(addy) => addy,
    Err(e) => return Err(format_err!(e))
  };
  let amount: CosmosCoin = match amount.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let r_state = state.read().await;
  if let Some(client) = &r_state.signing_client {
    match client.send(&address, vec!(amount), memo).await {
      Ok(_result) => Ok(()),
      Err(e) => Err(format_err!(e)),
    }
  } else {
    Err(String::from(
      "Client has not been initialized yet, connect with mnemonic to initialize",
    ))
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
      minor_to_major,
      major_to_minor,
      owns_gateway,
      owns_mixnode,
      bond_mixnode,
      unbond_mixnode,
      bond_gateway,
      unbond_gateway,
      delegate_to_mixnode,
      undelegate_from_mixnode,
      delegate_to_gateway,
      undelegate_from_gateway,
      send
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

export! {
  MixNode => "../src/types/rust/mixnode.ts",
  Coin => "../src/types/rust/coin.ts",
  Balance => "../src/types/rust/balance.ts",
  Gateway => "../src/types/rust/gateway.ts"
}
