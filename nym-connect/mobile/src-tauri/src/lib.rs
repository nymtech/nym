// Remove me after we've tidied up
#![allow(dead_code)]
#![allow(unused_imports)]

use nym_config_common::defaults::setup_env;
use std::sync::Arc;
use tauri::{App, Manager};
use tokio::sync::RwLock;

pub mod config;
mod error;
mod events;
mod logging;
mod models;
pub mod operations;
mod state;
mod tasks;

pub use state::State;

//#[cfg(target_os = "android")]
mod mobile;
//#[cfg(target_os = "android")]
pub use mobile::*;

pub type SetupHook = Box<dyn FnOnce(&mut App) -> Result<(), Box<dyn std::error::Error>> + Send>;

#[derive(Default)]
pub struct AppBuilder {
    setup: Option<SetupHook>,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn setup<F>(mut self, setup: F) -> Self
    where
        F: FnOnce(&mut App) -> Result<(), Box<dyn std::error::Error>> + Send + 'static,
    {
        self.setup.replace(Box::new(setup));
        self
    }

    pub fn run(self) {
        setup_env(None);

        println!("Starting up***");

        // As per breaking change description here
        // https://github.com/tauri-apps/tauri/blob/feac1d193c6d618e49916ad0707201f43d5cdd36/tooling/bundler/CHANGELOG.md
        if let Err(error) = fix_path_env::fix() {
            log::warn!("Failed to fix PATH: {error}");
        }

        let setup = self.setup;
        tauri::Builder::default()
            .manage(Arc::new(RwLock::new(State::default())))
            .invoke_handler(tauri::generate_handler![
                crate::config::get_config_file_location,
                crate::config::get_config_id,
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
                crate::operations::directory::get_gateways_detailed,
                crate::operations::export::export_keys,
            ])
            .setup(move |app| {
                if let Some(setup) = setup {
                    logging::setup_logging(app.app_handle())?;
                    (setup)(app)?;
                }
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
