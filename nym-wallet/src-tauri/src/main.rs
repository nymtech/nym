#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use mixnet_contract_common::{Gateway, MixNode};
use std::sync::Arc;
use tauri::Menu;
use tokio::sync::RwLock;
use validator_client::nymd::fee::helpers::Operation;

mod coin;
mod config;
mod error;
mod menu;
mod network;
mod operations;
mod state;
mod utils;
// temporarily until it is actually used
#[allow(unused)]
mod wallet_storage;

use crate::menu::AddDefaultSubmenus;
use crate::operations::mixnet;
use crate::operations::validator_api;
use crate::operations::vesting;

use crate::state::State;

fn main() {
  setup_logging();

  tauri::Builder::default()
    .manage(Arc::new(RwLock::new(State::default())))
    .invoke_handler(tauri::generate_handler![
      mixnet::account::connect_with_mnemonic,
      mixnet::account::create_new_account,
      mixnet::account::get_balance,
      mixnet::account::logout,
      mixnet::account::switch_network,
      mixnet::account::update_validator_urls,
      mixnet::admin::get_contract_settings,
      mixnet::admin::update_contract_settings,
      mixnet::bond::bond_gateway,
      mixnet::bond::bond_mixnode,
      mixnet::bond::gateway_bond_details,
      mixnet::bond::get_operator_rewards,
      mixnet::bond::mixnode_bond_details,
      mixnet::bond::unbond_gateway,
      mixnet::bond::unbond_mixnode,
      mixnet::bond::update_mixnode,
      mixnet::delegate::delegate_to_mixnode,
      mixnet::delegate::get_delegator_rewards,
      mixnet::delegate::get_reverse_mix_delegations_paged,
      mixnet::delegate::undelegate_from_mixnode,
      mixnet::send::send,
      utils::major_to_minor,
      utils::minor_to_major,
      utils::outdated_get_approximate_fee,
      utils::owns_gateway,
      utils::owns_mixnode,
      validator_api::status::gateway_core_node_status,
      validator_api::status::mixnode_core_node_status,
      validator_api::status::mixnode_inclusion_probability,
      validator_api::status::mixnode_reward_estimation,
      validator_api::status::mixnode_stake_saturation,
      validator_api::status::mixnode_status,
      vesting::bond::vesting_bond_gateway,
      vesting::bond::vesting_bond_mixnode,
      vesting::bond::vesting_unbond_gateway,
      vesting::bond::vesting_unbond_mixnode,
      vesting::bond::withdraw_vested_coins,
      vesting::delegate::vesting_delegate_to_mixnode,
      vesting::delegate::vesting_undelegate_from_mixnode,
      vesting::queries::delegated_free,
      vesting::queries::delegated_vesting,
      vesting::queries::get_account_info,
      vesting::queries::get_current_vesting_period,
      vesting::queries::locked_coins,
      vesting::queries::original_vesting,
      vesting::queries::spendable_coins,
      vesting::queries::vested_coins,
      vesting::queries::vesting_coins,
      vesting::queries::vesting_end_time,
      vesting::queries::vesting_get_gateway_pledge,
      vesting::queries::vesting_get_mixnode_pledge,
      vesting::queries::vesting_start_time,
    ])
    .menu(Menu::new().add_default_app_submenu_if_macos())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

fn setup_logging() {
  let mut log_builder = pretty_env_logger::formatted_timed_builder();
  if let Ok(s) = ::std::env::var("RUST_LOG") {
    log_builder.parse_filters(&s);
  } else {
    // default to 'Info'
    log_builder.filter(None, log::LevelFilter::Info);
  }

  log_builder
    .filter_module("hyper", log::LevelFilter::Warn)
    .filter_module("tokio_reactor", log::LevelFilter::Warn)
    .filter_module("reqwest", log::LevelFilter::Warn)
    .filter_module("mio", log::LevelFilter::Warn)
    .filter_module("want", log::LevelFilter::Warn)
    .filter_module("sled", log::LevelFilter::Warn)
    .filter_module("tungstenite", log::LevelFilter::Warn)
    .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
    .filter_module("rustls", log::LevelFilter::Warn)
    .filter_module("tokio_util", log::LevelFilter::Warn)
    .init();
}
