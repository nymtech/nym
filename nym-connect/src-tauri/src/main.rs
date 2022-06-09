#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use tauri::Manager;
use tauri::{Menu, SystemTrayEvent};
use tokio::sync::RwLock;

// temporarily until it is actually used
#[allow(unused)]
use crate::menu::AddDefaultSubmenus;
use crate::menu::{create_tray_menu, tray_menu_event_handler};
use crate::state::State;
use crate::window::window_toggle;

mod config;
mod error;
mod menu;
mod models;
mod operations;
mod state;
mod window;

fn main() {
    setup_logging();
    println!("Starting up...");

    // As per breaking change description here
    // https://github.com/tauri-apps/tauri/blob/feac1d193c6d618e49916ad0707201f43d5cdd36/tooling/bundler/CHANGELOG.md
    if let Err(error) = fix_path_env::fix() {
        log::warn!("Failed to fix PATH: {error}");
    }

    tauri::Builder::default()
        .manage(Arc::new(RwLock::new(State::new())))
        .invoke_handler(tauri::generate_handler![
            crate::operations::connection::connect::start_connecting,
            crate::operations::connection::disconnect::start_disconnecting,
            crate::operations::window::hide_window,
        ])
        .menu(Menu::new().add_default_app_submenu_if_macos())
        .system_tray(create_tray_menu())
        .on_system_tray_event(tray_menu_event_handler)
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
        .filter_module("handlebars", log::LevelFilter::Warn)
        .init();
}

#[cfg(test)]
mod test {
    ts_rs::export! {
      mixnet_contract_common::MixNode => "../src/types/rust/mixnode.ts",
    }
}
