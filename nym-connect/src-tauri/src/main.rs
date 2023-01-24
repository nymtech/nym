#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use config_common::defaults::setup_env;
use tauri::{Manager, Menu};
use tokio::sync::RwLock;

use crate::menu::AddDefaultSubmenus;
use crate::menu::{create_tray_menu, tray_menu_event_handler};
use crate::state::State;
use crate::window::window_toggle;

mod config;
mod error;
mod logging;
mod menu;
mod models;
mod operations;
mod state;
mod tasks;
mod window;

fn main() {
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
            crate::config::get_config_file_location,
            crate::config::get_config_id,
            crate::operations::connection::status::get_connection_status,
            crate::operations::connection::status::run_health_check,
            crate::operations::connection::connect::get_gateway,
            crate::operations::connection::connect::get_service_provider,
            crate::operations::connection::connect::set_gateway,
            crate::operations::connection::connect::set_service_provider,
            crate::operations::connection::connect::start_connecting,
            crate::operations::connection::disconnect::start_disconnecting,
            crate::operations::directory::get_services,
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
        .menu(Menu::os_default(&context.package_info().name).add_default_app_submenus())
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
