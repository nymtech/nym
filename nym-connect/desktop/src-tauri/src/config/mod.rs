// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::NymConnectPaths;
use crate::config::template::CONFIG_TEMPLATE;
use crate::{
    error::{BackendError, Result},
    state::State,
};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::config::Config as BaseClientConfig;
use nym_config_common::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_crypto::asymmetric::identity;
use nym_socks5_client_core::config::{Config as Socks5CoreConfig, Socks5};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tap::TapFallible;
use tokio::sync::RwLock;

mod persistence;
mod template;

static SOCKS5_CONFIG_ID: &str = "nym-connect";

// backwards compatibility : )
const DEFAULT_NYM_CONNECT_CLIENTS_DIR: &str = "socks5-clients";

/// Derive default path to nym connects's config file.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYM_CONNECT_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
        .join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to nym connects's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYM_CONNECT_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Config {
    pub socks5: Socks5CoreConfig,

    pub paths: NymConnectPaths,
}

impl NymConfigTemplate for Config {
    fn template() -> &'static str {
        CONFIG_TEMPLATE
    }
}

pub fn socks5_config_id_appended_with(gateway_id: &str) -> String {
    format!("{SOCKS5_CONFIG_ID}-{gateway_id}")
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S, provider_mix_address: S) -> Self {
        Config {
            socks5: Socks5CoreConfig::new(id.as_ref(), provider_mix_address.as_ref()),
            paths: NymConnectPaths::new_default(default_data_directory(id.as_ref())),
        }
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.socks5.base.client.id)
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = self.default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub async fn init(service_provider: &str, chosen_gateway_id: &str) -> Result<()> {
        log::info!("Initialising...");

        let service_provider = service_provider.to_owned();
        let chosen_gateway_id = chosen_gateway_id.to_owned();

        // The client initialization was originally not written for this use case, so there are
        // lots of ways it can panic. Until we have proper error handling in the init code for the
        // clients we'll catch any panics here by spawning a new runtime in a separate thread.
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime")
                .block_on(
                    async move { init_socks5_config(service_provider, chosen_gateway_id).await },
                )
        })
        .join()
        .map_err(|_| BackendError::InitializationPanic)??;

        log::info!("Configuration saved 🚀");
        Ok(())
    }
}

pub async fn init_socks5_config(provider_address: String, chosen_gateway_id: String) -> Result<()> {
    log::trace!("Initialising client...");

    // Append the gateway id to the name id that we store the config under
    let id = socks5_config_id_appended_with(&chosen_gateway_id);

    let already_init = if default_config_filepath(&id).exists() {
        eprintln!("SOCKS5 client \"{id}\" was already initialised before");
        true
    } else {
        false
    };

    // Future proofing. This flag exists for the other clients
    let user_wants_force_register = false;

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let register_gateway = !already_init || user_wants_force_register;

    log::trace!("Creating config for id: {id}");
    let mut config = Config::new(&id, &provider_address);

    if let Ok(raw_validators) = std::env::var(nym_config_common::defaults::var_names::NYM_API) {
        config.socks5.base.client.nym_api_urls = nym_config_common::parse_urls(&raw_validators);
    }

    let chosen_gateway_id = identity::PublicKey::from_base58_string(chosen_gateway_id)
        .map_err(|_| BackendError::UnableToParseGateway)?;

    // Setup gateway by either registering a new one, or reusing exiting keys
    let key_store = OnDiskKeys::new(config.paths.common_paths.keys_paths.clone());
    let gateway = nym_client_core::init::setup_gateway_from_config::<_>(
        &key_store,
        register_gateway,
        Some(chosen_gateway_id),
        &config.socks5.base,
        // TODO: another instance where this setting should probably get used
        false,
    )
    .await?;

    config.socks5.base.set_gateway_endpoint(gateway);

    let config_save_location = config.default_location();
    config.save_to_default_location().tap_err(|_| {
        log::error!("Failed to save the config file");
    })?;

    print_saved_config(&config);

    let address = nym_client_core::init::get_client_address_from_stored_ondisk_keys(
        &config.paths.common_paths.keys_paths,
        &config.socks5.base.client.gateway_endpoint,
    )?;
    log::info!("The address of this client is: {}", address);
    Ok(())
}

fn print_saved_config(config: &Config) {
    log::info!(
        "Saved configuration file to {}",
        config.default_location().display()
    );
    log::info!("Gateway id: {}", config.socks5.base.get_gateway_id());
    log::info!("Gateway owner: {}", config.socks5.base.get_gateway_owner());
    log::info!(
        "Gateway listener: {}",
        config.socks5.base.get_gateway_listener()
    );
    log::info!(
        "Service provider address: {}",
        config.socks5.socks5.provider_mix_address
    );
    log::info!(
        "Service provider port: {}",
        config.socks5.socks5.listening_port
    );
    log::info!("Client configuration completed.");
}
