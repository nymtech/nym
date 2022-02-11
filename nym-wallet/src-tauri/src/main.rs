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
  tauri::Builder::default()
    .manage(Arc::new(RwLock::new(State::default())))
    .invoke_handler(tauri::generate_handler![
      mixnet::account::connect_with_mnemonic,
      mixnet::account::create_new_account,
      mixnet::account::switch_network,
      mixnet::account::get_balance,
      mixnet::account::logout,
      mixnet::admin::get_contract_settings,
      mixnet::admin::update_contract_settings,
      mixnet::bond::bond_gateway,
      mixnet::bond::bond_mixnode,
      mixnet::bond::unbond_gateway,
      mixnet::bond::unbond_mixnode,
      mixnet::bond::update_mixnode,
      mixnet::bond::mixnode_bond_details,
      mixnet::bond::gateway_bond_details,
      mixnet::delegate::delegate_to_mixnode,
      mixnet::delegate::get_reverse_mix_delegations_paged,
      mixnet::delegate::undelegate_from_mixnode,
      mixnet::send::send,
      utils::outdated_get_approximate_fee,
      utils::major_to_minor,
      utils::minor_to_major,
      utils::owns_gateway,
      utils::owns_mixnode,
      vesting::bond::vesting_bond_gateway,
      vesting::bond::vesting_bond_mixnode,
      vesting::bond::vesting_unbond_gateway,
      vesting::bond::vesting_unbond_mixnode,
      vesting::delegate::vesting_delegate_to_mixnode,
      vesting::delegate::vesting_undelegate_from_mixnode,
      vesting::queries::locked_coins,
      vesting::queries::spendable_coins,
      vesting::queries::vesting_coins,
      vesting::queries::vested_coins,
      vesting::queries::vesting_start_time,
      vesting::queries::vesting_end_time,
      vesting::queries::original_vesting,
      vesting::queries::delegated_free,
      vesting::queries::delegated_vesting,
      vesting::queries::vesting_get_mixnode_pledge,
      vesting::queries::vesting_get_gateway_pledge,
      vesting::queries::get_current_vesting_period,
      validator_api::status::mixnode_core_node_status,
      validator_api::status::gateway_core_node_status,
      validator_api::status::mixnode_status,
      validator_api::status::mixnode_reward_estimation,
      validator_api::status::mixnode_stake_saturation,
      validator_api::status::mixnode_inclusion_probability,
    ])
    .menu(Menu::new().add_default_app_submenu_if_macos())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(test)]
mod test {
  ts_rs::export! {
    mixnet_contract_common::MixNode => "../src/types/rust/mixnode.ts",
    crate::coin::Coin => "../src/types/rust/coin.ts",
    crate::network::Network => "../src/types/rust/network.ts",
    crate::mixnet::account::Balance => "../src/types/rust/balance.ts",
    mixnet_contract_common::Gateway => "../src/types/rust/gateway.ts",
    crate::mixnet::send::TauriTxResult => "../src/types/rust/tauritxresult.ts",
    crate::mixnet::send::TransactionDetails => "../src/types/rust/transactiondetails.ts",
    validator_client::nymd::fee::helpers::Operation => "../src/types/rust/operation.ts",
    crate::coin::Denom => "../src/types/rust/denom.ts",
    crate::utils::DelegationResult => "../src/types/rust/delegationresult.ts",
    crate::mixnet::account::Account => "../src/types/rust/account.ts",
    crate::mixnet::account::CreatedAccount => "../src/types/rust/createdaccount.ts",
    crate::mixnet::admin::TauriContractStateParams => "../src/types/rust/stateparams.ts",
    validator_client::models::CoreNodeStatusResponse => "../src/types/corenodestatusresponse.ts",
    validator_client::models::MixnodeStatus => "../src/types/rust/mixnodestatus.ts",
    validator_client::models::MixnodeStatusResponse => "../src/types/rust/mixnodestatusresponse.ts",
    validator_client::models::RewardEstimationResponse => "../src/types/rust/rewardestimationresponse.ts",
    validator_client::models::StakeSaturationResponse => "../src/types/rust/stakesaturaionresponse.ts",
    validator_client::models::InclusionProbabilityResponse => "../src/types/rust/inclusionprobabilityresponse.ts",
    vesting_contract_common::Period => "../src/types/rust/period.ts",
    crate::vesting::PledgeData => "../src/types/rust/pledgedata.ts",
  }
}
