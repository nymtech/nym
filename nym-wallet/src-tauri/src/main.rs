#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{Manager, Menu};

use nym_mixnet_contract_common::{Gateway, MixNode};

use crate::menu::AddDefaultSubmenus;
use crate::operations::app;
use crate::operations::help;
use crate::operations::mixnet;
use crate::operations::nym_api;
use crate::operations::signatures;
use crate::operations::simulate;
use crate::operations::vesting;
use crate::state::WalletState;

mod config;
mod error;
mod log;
mod menu;
mod network_config;
mod operations;
mod platform_constants;
mod state;
mod utils;
mod wallet_storage;

#[allow(clippy::too_many_lines)]
fn main() {
    dotenvy::dotenv().ok();

    let context = tauri::generate_context!();
    tauri::Builder::default()
        .manage(WalletState::default())
        .invoke_handler(tauri::generate_handler![
            app::version::check_version,
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
            mixnet::account::rename_account_for_password,
            mixnet::account::show_mnemonic_for_account_in_password,
            mixnet::account::sign_in_with_password,
            mixnet::account::sign_in_with_password_and_account_id,
            mixnet::account::switch_network,
            mixnet::account::update_password,
            mixnet::account::validate_mnemonic,
            mixnet::admin::get_contract_settings,
            mixnet::admin::update_contract_settings,
            mixnet::bond::bond_gateway,
            mixnet::bond::bond_mixnode,
            mixnet::bond::update_pledge,
            mixnet::bond::pledge_more,
            mixnet::bond::decrease_pledge,
            mixnet::bond::gateway_bond_details,
            mixnet::bond::get_pending_operator_rewards,
            mixnet::bond::mixnode_bond_details,
            mixnet::bond::unbond_gateway,
            mixnet::bond::unbond_mixnode,
            mixnet::bond::update_mixnode_cost_params,
            mixnet::bond::update_mixnode_config,
            mixnet::bond::update_gateway_config,
            mixnet::bond::get_number_of_mixnode_delegators,
            mixnet::bond::get_mix_node_description,
            mixnet::bond::get_mixnode_avg_uptime,
            mixnet::bond::bond_nymnode,
            mixnet::bond::unbond_nymnode,
            mixnet::bond::nym_node_bond_details,
            mixnet::bond::get_nym_node_description,
            mixnet::bond::migrate_legacy_mixnode,
            mixnet::bond::migrate_legacy_gateway,
            mixnet::bond::update_nymnode_config,
            mixnet::bond::get_nymnode_performance,
            mixnet::delegate::delegate_to_mixnode,
            mixnet::delegate::get_pending_delegator_rewards,
            mixnet::delegate::get_pending_delegation_events,
            mixnet::delegate::get_delegation_summary,
            mixnet::delegate::get_all_mix_delegations,
            mixnet::delegate::undelegate_from_mixnode,
            mixnet::delegate::undelegate_all_from_mixnode,
            mixnet::interval::get_current_interval,
            mixnet::interval::get_pending_epoch_events,
            mixnet::interval::get_pending_interval_events,
            mixnet::rewards::claim_delegator_reward,
            mixnet::rewards::claim_operator_reward,
            mixnet::rewards::claim_locked_and_unlocked_delegator_reward,
            mixnet::rewards::get_current_rewarding_parameters,
            mixnet::send::send,
            mixnet::bond::get_mixnode_uptime,
            network_config::add_validator,
            network_config::get_nym_api_urls,
            network_config::get_nyxd_urls,
            network_config::remove_validator,
            network_config::select_nym_api_url,
            network_config::select_nyxd_url,
            network_config::reset_nyxd_url,
            network_config::get_default_nyxd_url,
            network_config::get_selected_nyxd_url,
            network_config::update_nyxd_urls,
            state::load_config_from_files,
            state::save_config_to_files,
            utils::owns_gateway,
            utils::owns_mixnode,
            utils::owns_nym_node,
            utils::get_env,
            utils::try_convert_pubkey_to_mix_id,
            utils::default_mixnode_cost_params,
            nym_api::status::compute_mixnode_reward_estimation,
            nym_api::status::gateway_core_node_status,
            nym_api::status::mixnode_core_node_status,
            nym_api::status::mixnode_inclusion_probability,
            nym_api::status::mixnode_reward_estimation,
            nym_api::status::mixnode_stake_saturation,
            nym_api::status::mixnode_status,
            nym_api::status::gateway_report,
            vesting::rewards::vesting_claim_delegator_reward,
            vesting::rewards::vesting_claim_operator_reward,
            vesting::bond::vesting_bond_gateway,
            vesting::bond::vesting_bond_mixnode,
            vesting::bond::vesting_update_pledge,
            vesting::bond::vesting_pledge_more,
            vesting::bond::vesting_decrease_pledge,
            vesting::bond::vesting_unbond_gateway,
            vesting::bond::vesting_unbond_mixnode,
            vesting::bond::vesting_update_mixnode_cost_params,
            vesting::bond::vesting_update_mixnode_config,
            vesting::bond::vesting_update_gateway_config,
            vesting::bond::withdraw_vested_coins,
            vesting::delegate::vesting_delegate_to_mixnode,
            vesting::delegate::vesting_undelegate_from_mixnode,
            vesting::migrate::migrate_vested_mixnode,
            vesting::migrate::migrate_vested_delegations,
            vesting::queries::get_account_info,
            vesting::queries::get_current_vesting_period,
            vesting::queries::locked_coins,
            vesting::queries::original_vesting,
            vesting::queries::spendable_coins,
            vesting::queries::spendable_vested_coins,
            vesting::queries::spendable_reward_coins,
            vesting::queries::get_historical_vesting_staking_reward,
            vesting::queries::get_spendable_vested_coins,
            vesting::queries::get_spendable_reward_coins,
            vesting::queries::get_delegated_coins,
            vesting::queries::get_pledged_coins,
            vesting::queries::get_withdrawn_coins,
            vesting::queries::get_staked_coins,
            vesting::queries::delegated_free,
            vesting::queries::delegated_vesting,
            vesting::queries::vested_coins,
            vesting::queries::vesting_coins,
            vesting::queries::vesting_end_time,
            vesting::queries::vesting_get_gateway_pledge,
            vesting::queries::vesting_get_mixnode_pledge,
            vesting::queries::vesting_start_time,
            simulate::admin::simulate_update_contract_settings,
            simulate::cosmos::simulate_send,
            simulate::cosmos::get_custom_fees,
            simulate::mixnet::simulate_bond_gateway,
            simulate::mixnet::simulate_unbond_gateway,
            simulate::mixnet::simulate_bond_mixnode,
            simulate::mixnet::simulate_update_pledge,
            simulate::mixnet::simulate_pledge_more,
            simulate::mixnet::simulate_unbond_mixnode,
            simulate::mixnet::simulate_update_mixnode_config,
            simulate::mixnet::simulate_update_mixnode_cost_params,
            simulate::mixnet::simulate_update_gateway_config,
            simulate::mixnet::simulate_delegate_to_mixnode,
            simulate::mixnet::simulate_undelegate_from_mixnode,
            simulate::vesting::simulate_vesting_delegate_to_mixnode,
            simulate::vesting::simulate_vesting_undelegate_from_mixnode,
            simulate::vesting::simulate_vesting_bond_gateway,
            simulate::vesting::simulate_vesting_unbond_gateway,
            simulate::vesting::simulate_vesting_bond_mixnode,
            simulate::vesting::simulate_vesting_update_pledge,
            simulate::vesting::simulate_vesting_pledge_more,
            simulate::vesting::simulate_vesting_unbond_mixnode,
            simulate::vesting::simulate_vesting_update_mixnode_config,
            simulate::vesting::simulate_vesting_update_gateway_config,
            simulate::vesting::simulate_vesting_update_mixnode_cost_params,
            simulate::vesting::simulate_withdraw_vested_coins,
            simulate::vesting::simulate_vesting_claim_delegator_reward,
            simulate::vesting::simulate_vesting_claim_operator_reward,
            simulate::mixnet::simulate_claim_delegator_reward,
            simulate::mixnet::simulate_claim_operator_reward,
            signatures::sign::sign,
            signatures::sign::verify,
            signatures::ed25519_signing_payload::generate_mixnode_bonding_msg_payload,
            signatures::ed25519_signing_payload::vesting_generate_mixnode_bonding_msg_payload,
            signatures::ed25519_signing_payload::generate_gateway_bonding_msg_payload,
            signatures::ed25519_signing_payload::generate_nym_node_bonding_msg_payload,
            signatures::ed25519_signing_payload::vesting_generate_gateway_bonding_msg_payload,
            help::log::help_log_toggle_window,
            app::window::create_main_window,
            app::window::create_auth_window,
            app::react::set_react_state,
            app::react::get_react_state,
        ])
        .menu(Menu::os_default(&context.package_info().name).add_default_app_submenus())
        .on_menu_event(|event| {
            if event.menu_item_id() == menu::SHOW_LOG_WINDOW {
                let _r = help::log::help_log_toggle_window(event.window().app_handle());
            }
        })
        .setup(|app| Ok(log::setup_logging(app.app_handle())?))
        .run(context)
        .expect("error while running tauri application");
}
