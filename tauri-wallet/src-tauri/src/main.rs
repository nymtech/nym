#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use mixnet_contract::{Gateway, MixNode};
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::export;
use validator_client::nymd::fee_helpers::Operation;

mod coin;
mod config;
mod error;
mod operations;
mod state;
mod utils;

use crate::operations::account::*;
use crate::operations::admin::*;
use crate::operations::bond::*;
use crate::operations::delegate::*;
use crate::operations::send::*;
use crate::utils::*;

use crate::state::State;

#[cfg(test)]
use crate::coin::{Coin, Denom};

#[macro_export]
macro_rules! format_err {
  ($e:expr) => {
    format!("line {}: {}", line!(), $e)
  };
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
      get_fee,
      get_state_params,
      update_state_params,
      get_reverse_mix_delegations_paged,
      get_reverse_gateway_delegations_paged,
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
  DelegationResult => "../src/types/rust/delegationresult.ts",
  Account => "../src/types/rust/account.ts",
  TauriStateParams => "../src/types/rust/stateparams.ts"
}
