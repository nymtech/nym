#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use config_common::defaults::setup_env;
use logging::setup_logging;
use tauri::Menu;
use tokio::sync::RwLock;

use crate::menu::AddDefaultSubmenus;
use crate::menu::{create_tray_menu, tray_menu_event_handler};
use crate::state::State;
use crate::window::window_toggle;
use std::path::PathBuf;

mod config;
mod error;
mod menu;
mod models;
mod operations;
mod state;
mod tasks;
mod window;

fn main() {
    setup_logging();

    //use qwerty environment
    //path may need changing for your set up
    let qwerty_environment = PathBuf::from("~/nym/envs/qa-qwerty.env");
    setup_env(Some(qwerty_environment));
    
    println!("Starting up...");

    // As per breaking change description here
    // https://github.com/tauri-apps/tauri/blob/feac1d193c6d618e49916ad0707201f43d5cdd36/tooling/bundler/CHANGELOG.md
    if let Err(error) = fix_path_env::fix() {
        log::warn!("Failed to fix PATH: {error}");
    }

    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(State::new())))
        .invoke_handler(tauri::generate_handler![
            crate::config::get_config_file_location,
            crate::config::get_config_id,
            crate::operations::connection::connect::get_gateway,
            crate::operations::connection::connect::get_service_provider,
            crate::operations::connection::connect::set_gateway,
            crate::operations::connection::connect::set_service_provider,
            crate::operations::connection::connect::start_connecting,
            crate::operations::connection::disconnect::start_disconnecting,
            crate::operations::directory::get_services,
            crate::operations::export::export_keys,
            crate::operations::window::hide_window,
        ])
        .menu(Menu::new().add_default_app_submenu_if_macos())
        .system_tray(create_tray_menu())
        .on_system_tray_event(tray_menu_event_handler)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
