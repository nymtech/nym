// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::NymConnectPaths;
use crate::config::template::CONFIG_TEMPLATE;
use crate::config::upgrade::try_upgrade_config;
use crate::error::{BackendError, Result};
use nym_client_core::client::base_client::non_wasm_helpers::setup_fs_gateways_storage;
use nym_client_core::client::base_client::storage::gateways_storage::{
    GatewayDetails, RemoteGatewayDetails,
};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::error::ClientCoreError;
use nym_client_core::init::generate_new_client_keys;
use nym_client_core::init::helpers::current_gateways;
use nym_client_core::init::types::{GatewaySelectionSpecification, GatewaySetup};
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_crypto::asymmetric::identity;
use nym_socks5_client_core::config::Config as Socks5CoreConfig;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, io};
use tap::TapFallible;

mod old_config_v1_1_13;
mod old_config_v1_1_20;
mod old_config_v1_1_20_2;
mod old_config_v1_1_30;
mod old_config_v1_1_33;
mod persistence;
mod template;
mod upgrade;
mod user_data;

pub use user_data::*;

static SOCKS5_CONFIG_ID: &str = "nym-connect";

// backwards compatibility : )
const DEFAULT_NYM_CONNECT_CLIENTS_DIR: &str = "socks5-clients";

/// Derive default path to clients's config directory.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYM_CONNECT_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to client's config file.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
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
    pub core: Socks5CoreConfig,

    pub storage_paths: NymConnectPaths,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

pub fn socks5_config_id_appended_with(gateway_id: &str) -> String {
    format!("{SOCKS5_CONFIG_ID}-{gateway_id}")
}

impl Config {
    pub fn new<S: AsRef<str>>(id: S, provider_mix_address: S) -> Self {
        Config {
            core: Socks5CoreConfig::new(
                id.as_ref(),
                env!("CARGO_PKG_VERSION"),
                provider_mix_address.as_ref(),
            ),
            storage_paths: NymConnectPaths::new_default(default_data_directory(id.as_ref())),
        }
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn default_location(&self) -> PathBuf {
        default_config_filepath(&self.core.base.client.id)
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
        // JS: why are we spawning a new thread here?
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

fn init_paths(id: &str) -> io::Result<()> {
    fs::create_dir_all(default_data_directory(id))?;
    fs::create_dir_all(default_config_directory(id))
}

fn try_extract_version_for_upgrade_failure(err: BackendError) -> Option<String> {
    if let BackendError::ClientCoreError {
        source: ClientCoreError::UnableToUpgradeConfigFile { new_version },
    } = err
    {
        Some(new_version)
    } else {
        None
    }
}

pub async fn init_socks5_config(provider_address: String, chosen_gateway_id: String) -> Result<()> {
    log::trace!("Initialising client...");

    // Append the gateway id to the name id that we store the config under
    let id = socks5_config_id_appended_with(&chosen_gateway_id);
    let _validated = identity::PublicKey::from_base58_string(&chosen_gateway_id)
        .map_err(|_| BackendError::UnableToParseGateway)?;

    let config_path = default_config_filepath(&id);
    let already_init = if config_path.exists() {
        // in case we're using old config, try to upgrade it
        // (if we're using the current version, it's a no-op)
        if let Err(err) = try_upgrade_config(&id).await {
            log::error!(
                "Failed to upgrade config file {}: {err}",
                config_path.display(),
            );
            return if let Some(failed_at_version) = try_extract_version_for_upgrade_failure(err) {
                Err(
                    BackendError::CouldNotUpgradeExistingConfigurationFileAtVersion {
                        file: config_path,
                        failed_at_version,
                    },
                )
            } else {
                Err(BackendError::CouldNotUpgradeExistingConfigurationFile { file: config_path })
            };
        };
        eprintln!("SOCKS5 client \"{id}\" was already initialised before");
        true
    } else {
        init_paths(&id)?;
        false
    };

    // // Future proofing. This flag exists for the other clients
    // let user_wants_force_register = false;

    log::trace!("Creating config for id: {id}");
    let mut config = Config::new(&id, &provider_address);

    if let Ok(raw_validators) = std::env::var(nym_config::defaults::var_names::NYM_API) {
        config.core.base.client.nym_api_urls = nym_config::parse_urls(&raw_validators);
    }

    let key_store = OnDiskKeys::new(config.storage_paths.common_paths.keys.clone());
    let details_store =
        setup_fs_gateways_storage(&config.storage_paths.common_paths.gateway_registrations).await?;

    // if this is a first time client with this particular id is initialised, generated long-term keys
    if !already_init {
        let mut rng = OsRng;
        generate_new_client_keys(&mut rng, &key_store).await?;
    }

    let gateway_setup = if !already_init {
        let selection_spec =
            GatewaySelectionSpecification::new(Some(chosen_gateway_id), None, false);
        let mut rng = rand::thread_rng();
        let available_gateways =
            current_gateways(&mut rng, &config.core.base.client.nym_api_urls).await?;
        GatewaySetup::New {
            specification: selection_spec,
            available_gateways,
            wg_tun_address: None,
        }
    } else {
        GatewaySetup::MustLoad {
            gateway_id: Some(chosen_gateway_id),
        }
    };

    let init_details =
        nym_client_core::init::setup_gateway(gateway_setup, &key_store, &details_store).await?;

    let GatewayDetails::Remote(gateway_details) = &init_details.gateway_registration.details else {
        return Err(ClientCoreError::UnexpectedPersistedCustomGatewayDetails)?;
    };

    config.save_to_default_location().tap_err(|_| {
        log::error!("Failed to save the config file");
    })?;

    print_saved_config(&config, gateway_details);

    let address = init_details.client_address();
    log::info!("The address of this client is: {address}");
    Ok(())
}

fn print_saved_config(config: &Config, gateway_details: &RemoteGatewayDetails) {
    log::info!(
        "Saved configuration file to {}",
        config.default_location().display()
    );
    log::info!("Gateway id: {}", gateway_details.gateway_id);
    if let Some(owner) = gateway_details.gateway_owner_address.as_ref() {
        log::info!("Gateway owner: {owner}");
    }
    log::info!("Gateway listener: {}", gateway_details.gateway_listener);
    log::info!(
        "Service provider address: {}",
        config.core.socks5.provider_mix_address
    );
    log::info!(
        "Service provider port: {}",
        config.core.socks5.bind_address.port()
    );
    log::info!("Client configuration completed.");
}
