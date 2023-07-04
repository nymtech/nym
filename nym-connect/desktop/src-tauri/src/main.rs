#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use nym_config::defaults::setup_env;
use tauri::Manager;
use tokio::sync::RwLock;

use crate::menu::{create_tray_menu, tray_menu_event_handler};
use crate::state::{is_medium_enabled, State};
use crate::window::window_toggle;

mod config;
mod error;
mod events;
mod logging;
mod menu;
mod models;
mod operations;
mod state;
mod tasks;
mod window;

fn main() {
    if is_medium_enabled() {
        println!("medium mode enabled");
        std::env::set_var("NYM_CONNECT_DISABLE_COVER", "1");
        std::env::set_var("NYM_CONNECT_ENABLE_MIXED_SIZE_PACKETS", "1");
        std::env::set_var("NYM_CONNECT_DISABLE_PER_HOP_DELAYS", "1");
    }
    setup_env(None);
    println!("Starting up...");

    // As per breaking change description here
    // https://github.com/tauri-apps/tauri/blob/feac1d193c6d618e49916ad0707201f43d5cdd36/tooling/bundler/CHANGELOG.md
    if let Err(error) = fix_path_env::fix() {
        log::warn!("Failed to fix PATH: {error}");
    }

    let context = tauri::generate_context!();
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(State::new())))
        .invoke_handler(tauri::generate_handler![
            crate::operations::config::get_config_file_location,
            crate::operations::config::get_config_id,
            crate::operations::common::get_env,
            crate::operations::connection::connect::get_gateway,
            crate::operations::connection::connect::get_service_provider,
            crate::operations::connection::connect::set_gateway,
            crate::operations::connection::connect::set_service_provider,
            crate::operations::connection::connect::start_connecting,
            crate::operations::connection::disconnect::start_disconnecting,
            crate::operations::connection::status::get_connection_health_check_status,
            crate::operations::connection::status::get_connection_status,
            crate::operations::connection::status::get_gateway_connection_status,
            crate::operations::connection::status::start_connection_health_check_task,
            crate::operations::connection::status::is_medium_mode_enabled,
            crate::operations::directory::get_services,
            crate::operations::directory::get_gateways_detailed,
            crate::operations::export::export_keys,
            crate::operations::window::hide_window,
            crate::operations::growth::test_and_earn::growth_tne_get_client_id,
            crate::operations::growth::test_and_earn::growth_tne_take_part,
            crate::operations::growth::test_and_earn::growth_tne_get_draws,
            crate::operations::growth::test_and_earn::growth_tne_ping,
            crate::operations::growth::test_and_earn::growth_tne_submit_wallet_address,
            crate::operations::growth::test_and_earn::growth_tne_enter_draw,
            crate::operations::growth::test_and_earn::growth_tne_toggle_window,
            crate::operations::help::log::help_log_toggle_window,
        ])
        .on_menu_event(|event| {
            if event.menu_item_id() == menu::SHOW_LOG_WINDOW {
                let _r = crate::operations::help::log::help_log_toggle_window(
                    event.window().app_handle(),
                );
            }
            if event.menu_item_id() == menu::CLEAR_STORAGE {
                let _r = crate::operations::help::storage::help_clear_storage(
                    event.window().app_handle(),
                );
            }
        })
        .setup(|app| Ok(crate::logging::setup_logging(app.app_handle())?))
        .system_tray(create_tray_menu())
        .on_system_tray_event(tray_menu_event_handler)
        .run(context)
        .expect("error while running tauri application");
}
