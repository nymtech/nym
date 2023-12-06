// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Result;
use tauri::api::path::{config_dir, data_dir};
use tokio::sync::Mutex;
use tracing::info;

mod commands;
mod error;
mod fs;
mod states;

use commands::*;
use states::app::AppState;

use crate::fs::config::AppConfig;
use crate::fs::data::AppData;
use crate::fs::storage::AppStorage;

const APP_DIR: &str = "nymvpn";
const APP_DATA_FILE: &str = "app-data.toml";
const APP_CONFIG_FILE: &str = "config.toml";

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // uses RUST_LOG value for logging level
    // eg. RUST_LOG=tauri=debug,nymvpn_ui=trace
    tracing_subscriber::fmt::init();

    let mut app_data_path = data_dir().ok_or(anyhow!("Failed to retrieve data directory"))?;
    app_data_path.push(APP_DIR);
    let app_data_store = AppStorage::<AppData>::new(app_data_path, APP_DATA_FILE, None);

    let mut app_config_path = config_dir().ok_or(anyhow!("Failed to retrieve config directory"))?;
    app_config_path.push(APP_DIR);
    let app_config_store = AppStorage::<AppConfig>::new(app_config_path, APP_CONFIG_FILE, None);

    info!("Starting tauri app");

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState::default())))
        .manage(Arc::new(Mutex::new(app_data_store)))
        .manage(Arc::new(Mutex::new(app_config_store)))
        .setup(|_app| {
            info!("app setup");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connection::set_vpn_mode,
            connection::get_connection_state,
            connection::connect,
            connection::disconnect,
            connection::get_connection_start_time,
            app_data::get_app_data,
            app_data::set_app_data,
            app_data::set_ui_theme,
            app_data::get_node_countries,
            node_location::set_node_location,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
