// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Result};
use tauri::api::path::{config_dir, data_dir};
use tokio::sync::Mutex;
use tracing::{debug, info};

use commands::*;
use states::app::AppState;

use nym_vpn_lib::{
    gateway_client::Config as GatewayClientConfig,
    nym_config::{self, OptionalSet},
    NymVPN,
};

use crate::fs::{config::AppConfig, data::AppData, storage::AppStorage};

mod commands;
mod error;
mod fs;
mod states;

const APP_DIR: &str = "nymvpn";
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

fn setup_gateway_client_config(private_key: Option<&str>, nym_api: &str) -> GatewayClientConfig {
    let mut config = GatewayClientConfig::default()
        // .with_custom_api_url(nym_config::defaults::mainnet::NYM_API.parse().unwrap())
        .with_custom_api_url(nym_api.parse().unwrap())
        // Read in the environment variable NYM_API if it exists
        .with_optional_env(GatewayClientConfig::with_custom_api_url, None, "NYM_API");
    info!("Using nym-api: {}", config.api_url());

    if let Some(key) = private_key {
        config = config.with_local_private_key(key.into());
    }
    config
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

    let app_config = app_config_store.read().await?;
    debug!("app_config: {app_config:?}");

    // Read the env variables in the provided file and export them all to the local environment.
    // TODO: consider reading in the environment from the config file instead.
    // nym_config::defaults::setup_env(env::args().nth(1).map(PathBuf::from).as_ref());
    // TEMPORARY: hardcode the path to the env file until we can read it from the config file
    nym_config::defaults::setup_env(Some("/home/dev/src/nym/nym/envs/foxyfox.env".parse::<PathBuf>().unwrap()));

    let nym_vpn = {
        let mut nym_vpn = NymVPN::new(&app_config.entry_gateway, &app_config.exit_router);
        nym_vpn.gateway_config = setup_gateway_client_config(None, &app_config.nym_api);
        nym_vpn
    };

    info!("Starting tauri app");

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(AppState::default())))
        .manage(Arc::new(Mutex::new(app_data_store)))
        .manage(Arc::new(Mutex::new(app_config_store)))
        .manage(Arc::new(Mutex::new(nym_vpn)))
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
