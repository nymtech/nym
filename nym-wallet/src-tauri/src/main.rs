#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use mixnet_contract_common::{Gateway, MixNode};
use std::sync::Arc;
use tauri::Menu;
use tokio::sync::RwLock;

mod config;
mod error;
mod menu;
mod network_config;
mod operations;
mod platform_constants;
mod state;
mod utils;
mod wallet_storage;

use crate::menu::AddDefaultSubmenus;
use crate::operations::mixnet;
use crate::operations::simulate;
use crate::operations::validator_api;
use crate::operations::vesting;

use crate::state::State;

#[allow(clippy::too_many_lines)]
fn main() {
    dotenv::dotenv().ok();
    setup_logging();

    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(State::default())))
        .invoke_handler(tauri::generate_handler![
            mixnet::account::add_account_for_password,
            mixnet::account::archive_wallet_file,
            mixnet::account::connect_with_mnemonic,
            mixnet::account::create_new_mnemonic,
            mixnet::account::create_password,
            mixnet::account::does_password_file_exist,
            mixnet::account::get_balance,
            mixnet::account::list_accounts,
            mixnet::account::logout,
            mixnet::account::remove_account_for_password,
            mixnet::account::remove_password,
            mixnet::account::show_mnemonic_for_account_in_password,
            mixnet::account::sign_in_with_password,
            mixnet::account::sign_in_with_password_and_account_id,
            mixnet::account::switch_network,
            mixnet::account::validate_mnemonic,
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
            mixnet::delegate::get_pending_delegation_events,
            mixnet::delegate::get_delegation_summary,
            mixnet::delegate::get_all_pending_delegation_events,
            mixnet::delegate::get_all_mix_delegations,
            mixnet::delegate::undelegate_from_mixnode,
            mixnet::delegate::undelegate_all_from_mixnode,
            mixnet::epoch::get_current_epoch,
            mixnet::rewards::claim_delegator_reward,
            mixnet::rewards::claim_operator_reward,
            mixnet::rewards::compound_operator_reward,
            mixnet::rewards::compound_delegator_reward,
            mixnet::rewards::claim_locked_and_unlocked_delegator_reward,
            mixnet::rewards::compound_locked_and_unlocked_delegator_reward,
            mixnet::send::send,
            network_config::add_validator,
            network_config::get_validator_api_urls,
            network_config::get_validator_nymd_urls,
            network_config::remove_validator,
            network_config::select_validator_api_url,
            network_config::select_validator_nymd_url,
            network_config::update_validator_urls,
            state::load_config_from_files,
            state::save_config_to_files,
            utils::owns_gateway,
            utils::owns_mixnode,
            utils::get_env,
            utils::get_old_and_incorrect_hardcoded_fee,
            validator_api::status::gateway_core_node_status,
            validator_api::status::mixnode_core_node_status,
            validator_api::status::mixnode_inclusion_probability,
            validator_api::status::mixnode_reward_estimation,
            validator_api::status::mixnode_stake_saturation,
            validator_api::status::mixnode_status,
            vesting::rewards::vesting_claim_delegator_reward,
            vesting::rewards::vesting_claim_operator_reward,
            vesting::rewards::vesting_compound_operator_reward,
            vesting::rewards::vesting_compound_delegator_reward,
            vesting::bond::vesting_bond_gateway,
            vesting::bond::vesting_bond_mixnode,
            vesting::bond::vesting_unbond_gateway,
            vesting::bond::vesting_unbond_mixnode,
            vesting::bond::vesting_update_mixnode,
            vesting::bond::withdraw_vested_coins,
            vesting::delegate::get_pending_vesting_delegation_events,
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
            simulate::admin::simulate_update_contract_settings,
            simulate::cosmos::simulate_send,
            simulate::mixnet::simulate_bond_gateway,
            simulate::mixnet::simulate_unbond_gateway,
            simulate::mixnet::simulate_bond_mixnode,
            simulate::mixnet::simulate_unbond_mixnode,
            simulate::mixnet::simulate_update_mixnode,
            simulate::mixnet::simulate_delegate_to_mixnode,
            simulate::mixnet::simulate_undelegate_from_mixnode,
            simulate::vesting::simulate_vesting_delegate_to_mixnode,
            simulate::vesting::simulate_vesting_undelegate_from_mixnode,
            simulate::vesting::simulate_vesting_bond_gateway,
            simulate::vesting::simulate_vesting_unbond_gateway,
            simulate::vesting::simulate_vesting_bond_mixnode,
            simulate::vesting::simulate_vesting_unbond_mixnode,
            simulate::vesting::simulate_vesting_update_mixnode,
            simulate::vesting::simulate_withdraw_vested_coins,
            simulate::vesting::simulate_vesting_claim_delegator_reward,
            simulate::vesting::simulate_vesting_claim_operator_reward,
            simulate::vesting::simulate_vesting_compound_operator_reward,
            simulate::vesting::simulate_vesting_compound_delegator_reward,
            simulate::mixnet::simulate_claim_delegator_reward,
            simulate::mixnet::simulate_claim_operator_reward,
            simulate::mixnet::simulate_compound_operator_reward,
            simulate::mixnet::simulate_compound_delegator_reward,
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

    if ::std::env::var("RUST_TRACE_OPERATIONS").is_ok() {
        log_builder.filter_module("nym_wallet::operations", log::LevelFilter::Trace);
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
