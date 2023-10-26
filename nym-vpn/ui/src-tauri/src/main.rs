// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;
use tracing::info;

mod commands;
mod error;
mod states;

use commands::*;
use states::app::AppState;

fn main() -> Result<()> {
    dotenvy::dotenv()?;

    // uses RUST_LOG value for logging level
    // eg. RUST_LOG=tauri=debug,nymvpn_ui=trace
    tracing_subscriber::fmt::init();

    info!("Starting tauri app");

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState::default())))
        .invoke_handler(tauri::generate_handler![
            greet,
            connection::get_connection_state,
            connection::connect,
            connection::disconnect
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
