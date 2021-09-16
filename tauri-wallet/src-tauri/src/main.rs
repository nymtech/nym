#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use bip39::{Language, Mnemonic};
use cosmwasm_std::Coin as CosmWasmCoin;
use error::BackendError;
use mixnet_contract::{Gateway, MixNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;
use tendermint_rpc::endpoint::broadcast::tx_commit::Response;
use tokio::sync::RwLock;
use ts_rs::{export, TS};
use validator_client::nymd::fee_helpers::Operation;
use validator_client::nymd::{NymdClient, SigningNymdClient, AccountId, CosmosCoin};

mod coin;
mod config;
mod error;
mod state;

use crate::state::State;

use crate::coin::{Coin, Denom};
use crate::config::Config;

#[macro_export]
macro_rules! format_err {
  ($e:expr) => {
    format!("line {}: {}", line!(), $e)
  };
}

#[derive(TS, Serialize, Deserialize)]
struct DelegationResult {
  source_address: String,
  target_address: String,
  amount: Option<Coin>,
}

#[derive(TS, Serialize, Deserialize)]
struct Balance {
  coin: Coin,
  printable_balance: String,
}

#[derive(Deserialize, Serialize, TS)]
struct TauriTxResult {
  code: u32,
  gas_wanted: u64,
  gas_used: u64,
  block_height: u64,
  details: TransactionDetails,
}

#[derive(Deserialize, Serialize, TS)]
struct TransactionDetails {
  from_address: String,
  to_address: String,
  amount: Coin,
}

impl TauriTxResult {
  fn new(t: Response, details: TransactionDetails) -> TauriTxResult {
    TauriTxResult {
      code: t.check_tx.code.value(),
      gas_wanted: t.check_tx.gas_wanted.value(),
      gas_used: t.check_tx.gas_used.value(),
      block_height: t.height.value(),
      details,
    }
  }
}

// TODO these should be more explicit
#[tauri::command]
fn major_to_minor(amount: &str) -> Result<Coin, String> {
  let coin = Coin::new(amount, &Denom::Major);
  Ok(coin.to_minor())
}

#[tauri::command]
fn minor_to_major(amount: &str) -> Result<Coin, String> {
  let coin = Coin::new(amount, &Denom::Minor);
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
    client = _connect_with_mnemonic(mnemonic, &r_state.config());
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
  w_state.set_client(client);

  Ok(ret)
}

#[tauri::command]
async fn get_balance(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<Balance, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.get_balance(client.address()).await {
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
    Ok(None) => Err(format!(
      "No balance available for address {}",
      client.address()
    )),
    Err(e) => Err(BackendError::from(e).to_string()),
  }
}

#[tauri::command]
async fn owns_mixnode(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<bool, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.owns_mixnode(client.address()).await {
    Ok(o) => Ok(o),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn owns_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<bool, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.owns_gateway(client.address()).await {
    Ok(o) => Ok(o),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn unbond_mixnode(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.unbond_mixnode().await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
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
  let client = r_state.client()?;
  match client.bond_mixnode(mixnode, bond).await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn delegate_to_mixnode(
  identity: &str,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, String> {
  let r_state = state.read().await;
  let bond: CosmWasmCoin = match amount.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let client = r_state.client()?;
  match client.delegate_to_mixnode(identity, &bond).await {
    Ok(_result) => Ok(DelegationResult {
      source_address: client.address().to_string(),
      target_address: identity.to_string(),
      amount: Some(bond.into()),
    }),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn undelegate_from_mixnode(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.remove_mixnode_delegation(identity).await {
    Ok(_result) => Ok(DelegationResult {
      source_address: client.address().to_string(),
      target_address: identity.to_string(),
      amount: None,
    }),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn delegate_to_gateway(
  identity: &str,
  amount: Coin,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, String> {
  let r_state = state.read().await;
  let bond: CosmWasmCoin = match amount.try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let client = r_state.client()?;
  match client.delegate_to_gateway(identity, &bond).await {
    Ok(_result) => Ok(DelegationResult {
      source_address: client.address().to_string(),
      target_address: identity.to_string(),
      amount: Some(bond.into()),
    }),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn undelegate_from_gateway(
  identity: &str,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<DelegationResult, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.remove_gateway_delegation(identity).await {
    Ok(_result) => Ok(DelegationResult {
      source_address: client.address().to_string(),
      target_address: identity.to_string(),
      amount: None,
    }),
    Err(e) => Err(format_err!(e)),
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
  let client = r_state.client()?;
  match client.bond_gateway(gateway, bond).await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn unbond_gateway(state: tauri::State<'_, Arc<RwLock<State>>>) -> Result<(), String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.unbond_gateway().await {
    Ok(_result) => Ok(()),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn send(
  address: &str,
  amount: Coin,
  memo: String,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<TauriTxResult, String> {
  let address = match AccountId::from_str(address) {
    Ok(addy) => addy,
    Err(e) => return Err(format_err!(e)),
  };
  let cosmos_amount: CosmosCoin = match amount.clone().try_into() {
    Ok(b) => b,
    Err(e) => return Err(format_err!(e)),
  };
  let r_state = state.read().await;
  let client = r_state.client()?;
  match client.send(&address, vec![cosmos_amount], memo).await {
    Ok(result) => Ok(TauriTxResult::new(
      result,
      TransactionDetails {
        from_address: client.address().to_string(),
        to_address: address.to_string(),
        amount,
      },
    )),
    Err(e) => Err(format_err!(e)),
  }
}

#[tauri::command]
async fn get_fee(
  operation: Operation,
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<Coin, String> {
  let r_state = state.read().await;
  let client = r_state.client()?;
  let fee = client.get_fee(operation);
  let mut coin = Coin::new("0", &Denom::Major);
  for f in fee.amount {
    coin = coin + f.into();
  }

  Ok(coin)
}

#[tauri::command]
async fn create_new_account(
  state: tauri::State<'_, Arc<RwLock<State>>>,
) -> Result<HashMap<&str, String>, String> {
  let mnemonic = random_mnemonic();
  let mut client = connect_with_mnemonic(mnemonic.to_string(), state).await?;
  client.insert("mnemonic", mnemonic.to_string());
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
      send,
      create_new_account,
      get_fee
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

export! {
  MixNode => "../src/types/rust/mixnode.ts",
  Coin => "../src/types/rust/coin.ts",
  Balance => "../src/types/rust/balance.ts",
  Gateway => "../src/types/rust/gateway.ts",
  TauriTxResult => "../src/types/rust/tauritxresult.ts",
  TransactionDetails => "../src/types/rust/transactiondetails.ts",
  Operation => "../src/types/rust/operation.ts",
  Denom => "../src/types/rust/denom.ts",
  DelegationResult => "../src/types/rust/delegationresult.ts"
}
