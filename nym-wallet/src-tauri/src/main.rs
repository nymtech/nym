#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use mixnet_contract::{Gateway, MixNode};
use std::sync::Arc;
use tauri::Menu;
use tokio::sync::RwLock;
use validator_client::nymd::fee::helpers::Operation;

mod coin;
mod config;
mod error;
mod menu;
mod operations;
mod state;
mod utils;

use crate::menu::AddDefaultSubmenus;
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
      send,
      create_new_account,
      get_approximate_fee,
      get_contract_settings,
      update_contract_settings,
      get_reverse_mix_delegations_paged,
    ])
    .menu(Menu::new().add_default_app_submenu_if_macos())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(test)]
mod test {
  ts_rs::export! {
    mixnet_contract::MixNode => "../src/types/rust/mixnode.ts",
    crate::Coin => "../src/types/rust/coin.ts",
    crate::Balance => "../src/types/rust/balance.ts",
    mixnet_contract::Gateway => "../src/types/rust/gateway.ts",
    crate::TauriTxResult => "../src/types/rust/tauritxresult.ts",
    crate::TransactionDetails => "../src/types/rust/transactiondetails.ts",
    validator_client::nymd::fee::helpers::Operation => "../src/types/rust/operation.ts",
    crate::Denom => "../src/types/rust/denom.ts",
    crate::DelegationResult => "../src/types/rust/delegationresult.ts",
    crate::Account => "../src/types/rust/account.ts",
    crate::TauriContractStateParams => "../src/types/rust/stateparams.ts"
  }
}
