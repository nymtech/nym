// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, sync::Arc};

use anyhow::{anyhow, Context, Result};
use tauri::api::path::{config_dir, data_dir};
use tokio::{fs::try_exists, sync::Mutex};
use tracing::{debug, error, info};

use commands::*;
use states::app::AppState;

use nym_vpn_lib::nym_config;

use crate::fs::{config::AppConfig, data::AppData, storage::AppStorage};

mod commands;
mod country;
mod error;
mod fs;
mod states;
mod vpn_client;

const APP_DIR: &str = "nym-vpn";
const APP_DATA_FILE: &str = "app-data.toml";
const APP_CONFIG_FILE: &str = "config.toml";

pub fn setup_logging() {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()
        .unwrap()
        .add_directive("hyper::proto=info".parse().unwrap())
        .add_directive("netlink_proto=info".parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    setup_logging();

    let app_data_store = {
        let mut app_data_path =
            data_dir().ok_or(anyhow!("Failed to retrieve data directory path"))?;
        app_data_path.push(APP_DIR);
        AppStorage::<AppData>::new(app_data_path, APP_DATA_FILE, None)
    };
    debug!("app_data_store: {}", app_data_store.full_path.display());

    let app_config_store = {
        let mut app_config_path =
            config_dir().ok_or(anyhow!("Failed to retrieve config directory path"))?;
        app_config_path.push(APP_DIR);
        AppStorage::<AppConfig>::new(app_config_path, APP_CONFIG_FILE, None)
    };
    debug!(
        "app_config_store: {}",
        &app_config_store.full_path.display()
    );

    let app_data = app_data_store.read().await?;
    debug!("app_data: {app_data:?}");
    let app_config = app_config_store.read().await?;
    debug!("app_config: {app_config:?}");

    // check for the existence of the env_config_file if provided
    if let Some(env_config_file) = &app_config.env_config_file {
        debug!("provided env_config_file: {}", env_config_file.display());
        if !(try_exists(env_config_file)
            .await
            .context("an error happened while trying to read env_config_file `{}`")?)
        {
            let err_message = format!(
                "app config, env_config_file `{}`: file not found",
                env_config_file.display()
            );
            error!(err_message);
            return Err(anyhow!(err_message));
        }
    }

    // Read the env variables in the provided file and export them all to the local environment.
    nym_config::defaults::setup_env(app_config.env_config_file);

    info!("Starting tauri app");

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState::from(&app_data))))
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
            app_data::set_entry_location_selector,
            app_data::get_node_countries,
            app_data::set_root_font_size,
            node_location::set_node_location,
            node_location::get_default_node_location,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
