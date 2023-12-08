// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Result;
use nym_vpn_lib::NymVPN;
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
use nym_vpn_lib::gateway_client::Config as GatewayConfig;
use nym_vpn_lib::nym_config::defaults::var_names::NYM_API;
use nym_vpn_lib::nym_config::OptionalSet;

const APP_DIR: &str = "nymvpn";
const APP_DATA_FILE: &str = "app-data.toml";
const APP_CONFIG_FILE: &str = "config.toml";

fn setup_gateway_config(private_key: Option<&str>, config: GatewayConfig) -> GatewayConfig {
    let mut config = config.with_optional_env(GatewayConfig::with_custom_api_url, None, NYM_API);
    if let Some(key) = private_key {
        config = config.with_local_private_key(key.into());
    }
    config
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // uses RUST_LOG value for logging level
    // eg. RUST_LOG=tauri=debug,nymvpn_ui=trace
    tracing_subscriber::fmt::init();

    let mut app_data_path = data_dir().ok_or(anyhow!("Failed to retrieve data directory path"))?;
    app_data_path.push(APP_DIR);
    let app_data_store = AppStorage::<AppData>::new(app_data_path, APP_DATA_FILE, None);

    let mut app_config_path =
        config_dir().ok_or(anyhow!("Failed to retrieve config directory path"))?;
    app_config_path.push(APP_DIR);
    let app_config_store = AppStorage::<AppConfig>::new(app_config_path, APP_CONFIG_FILE, None);

    let app_config = app_config_store.read().await?;
    // let gateway_config =
    //     setup_gateway_config(app_config.private_key.as_deref(), GatewayConfig::default());

    // This should only really need to be set if we're running on not-mainnet. By default we should
    // use the hardcoded stuff for mainnet, just like nym-connect.
    let network_env = Some("/home/foobar/src/nym/nym/envs/qa.env");
    nym_vpn_lib::nym_config::defaults::setup_env(network_env);

    let gateway_config = GatewayConfig::default().with_optional_env(
        GatewayConfig::with_custom_api_url,
        None,
        "NYM_API",
    );

    let mut nym_vpn = NymVPN::new(&app_config.entry_gateway, &app_config.exit_router);
    nym_vpn.gateway_config = gateway_config;

    // let nym_vpn = NymVPN {
    //     gateway_config,
    //     mixnet_client_path: app_config.mixnet_client_path,
    //     entry_gateway: app_config.entry_gateway,
    //     exit_router: app_config.exit_router,
    //     enable_wireguard: app_config.enable_wireguard.unwrap_or(false),
    //     private_key: app_config.private_key,
    //     ip: app_config.ip,
    //     mtu: app_config.mtu,
    //     disable_routing: app_config.disable_routing.unwrap_or(false),
    //     enable_two_hop: app_config.enable_two_hop.unwrap_or(false),
    //     enable_poisson_rate: app_config.enable_poisson_rate.unwrap_or(true),
    // };

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
