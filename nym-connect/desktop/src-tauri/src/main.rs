#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::env;
use std::sync::Arc;

use nym_config::defaults::setup_env;
use tauri::Manager;
use tokio::sync::RwLock;

use crate::config::UserData;
use crate::menu::{create_tray_menu, tray_menu_event_handler};
use crate::state::State;
use crate::window::window_toggle;

mod config;
mod constants;
mod error;
mod events;
mod logging;
mod menu;
mod models;
mod monitoring;
mod operations;
mod state;
mod tasks;
mod window;

fn main() {
    dotenvy::dotenv().ok();
    setup_env(None);
    println!("Starting up...");

    // As per breaking change description here
    // https://github.com/tauri-apps/tauri/blob/feac1d193c6d618e49916ad0707201f43d5cdd36/tooling/bundler/CHANGELOG.md
    if let Err(error) = fix_path_env::fix() {
        println!("Failed to fix PATH: {error}");
    }

    let user_data = UserData::read().unwrap_or_else(|e| {
        println!("{}", e);
        println!("Fallback to default");
        UserData::default()
    });

    let monitoring = user_data.monitoring.unwrap_or(false);
    let mut _sentry_guard;

    if monitoring {
        match monitoring::init() {
            Ok(guard) => {
                println!("Monitoring and error reporting enabled");

                // we must keep the sentry guard in scope during app lifetime
                _sentry_guard = guard;
            }
            Err(e) => {
                println!("Unable to init monitoring: {e}");
            }
        }
    }

    let context = tauri::generate_context!();
    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(State::new(user_data))))
        .invoke_handler(tauri::generate_handler![
            crate::operations::config::get_config_file_location,
            crate::operations::config::get_config_id,
            crate::operations::common::get_env,
            crate::operations::common::get_user_data,
            crate::operations::common::set_monitoring,
            crate::operations::common::set_privacy_level,
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
            crate::operations::directory::get_services,
            crate::operations::directory::get_gateways,
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
        .setup(move |app| Ok(crate::logging::setup_logging(app.app_handle(), monitoring)?))
        .system_tray(create_tray_menu())
        .on_system_tray_event(tray_menu_event_handler)
        .run(context)
        .expect("error while running tauri application");
}
