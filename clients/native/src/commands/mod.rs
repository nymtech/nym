// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::client::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::client::config::old_config_v1_1_20_2::ConfigV1_1_20_2;
use crate::client::config::{BaseClientConfig, Config};
use crate::error::ClientError;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use log::{error, info};
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_client_core::client::base_client::storage::gateway_details::{
    OnDiskGatewayDetails, PersistedGatewayDetails,
};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::config::GatewayEndpointConfig;
use nym_client_core::error::ClientCoreError;
use nym_config::OptionalSet;
use std::error::Error;
use std::net::IpAddr;
use std::sync::OnceLock;

pub(crate) mod build_info;
pub(crate) mod import_credential;
pub(crate) mod init;
pub(crate) mod run;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[clap(long)]
    pub(crate) no_banner: bool,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialise a Nym client. Do this first!
    Init(init::Init),

    /// Run the Nym client with provided configuration client optionally overriding set parameters
    Run(run::Run),

    /// Show build information of this binary
    BuildInfo(build_info::BuildInfo),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    nym_apis: Option<Vec<url::Url>>,
    disable_socket: Option<bool>,
    port: Option<u16>,
    host: Option<IpAddr>,
    fastmode: bool,
    no_cover: bool,
    nyxd_urls: Option<Vec<url::Url>>,
    enabled_credentials_mode: Option<bool>,
}

pub(crate) async fn execute(args: Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bin_name = "nym-native-client";

    match args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::BuildInfo(m) => build_info::execute(m),
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
}

pub(crate) fn override_config(config: Config, args: OverrideConfig) -> Config {
    config
        .with_optional(Config::with_disabled_socket, args.disable_socket)
        .with_base(
            BaseClientConfig::with_high_default_traffic_volume,
            args.fastmode,
        )
        .with_base(BaseClientConfig::with_disabled_cover_traffic, args.no_cover)
        .with_optional(Config::with_port, args.port)
        .with_optional(Config::with_host, args.host)
        .with_optional_custom_env_ext(
            BaseClientConfig::with_custom_nym_apis,
            args.nym_apis,
            nym_network_defaults::var_names::NYM_API,
            nym_config::parse_urls,
        )
        .with_optional_custom_env_ext(
            BaseClientConfig::with_custom_nyxd,
            args.nyxd_urls,
            nym_network_defaults::var_names::NYXD,
            nym_config::parse_urls,
        )
        .with_optional_ext(
            BaseClientConfig::with_disabled_credentials,
            args.enabled_credentials_mode.map(|b| !b),
        )
}

fn persist_gateway_details(
    config: &Config,
    details: GatewayEndpointConfig,
) -> Result<(), ClientError> {
    let details_store =
        OnDiskGatewayDetails::new(&config.storage_paths.common_paths.gateway_details);
    let keys_store = OnDiskKeys::new(config.storage_paths.common_paths.keys.clone());
    let shared_keys = keys_store.ephemeral_load_gateway_keys().map_err(|source| {
        ClientError::ClientCoreError(ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })
    })?;
    let persisted_details = PersistedGatewayDetails::new(details.into(), Some(&shared_keys))?;
    details_store
        .store_to_disk(&persisted_details)
        .map_err(|source| {
            ClientError::ClientCoreError(ClientCoreError::GatewayDetailsStoreError {
                source: Box::new(source),
            })
        })
}

fn try_upgrade_v1_1_13_config(id: &str) -> Result<bool, ClientError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.13 (which is incompatible with the next step, i.e. 1.1.19)
    let Ok(old_config) = OldConfigV1_1_13::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.13 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20 = old_config.into();
    let updated_step2: ConfigV1_1_20_2 = updated_step1.into();
    let (updated, gateway_config) = updated_step2.upgrade()?;
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_v1_1_20_config(id: &str) -> Result<bool, ClientError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated_step1: ConfigV1_1_20_2 = old_config.into();
    let (updated, gateway_config) = updated_step1.upgrade()?;
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_v1_1_20_2_config(id: &str) -> Result<bool, ClientError> {
    // explicitly load it as v1.1.20_2 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20_2::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20_2 config template.");
    info!("It is going to get updated to the current specification.");

    let (updated, gateway_config) = old_config.upgrade()?;
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_config(id: &str) -> Result<(), ClientError> {
    if try_upgrade_v1_1_13_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_config(id)? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_2_config(id)? {
        return Ok(());
    }

    Ok(())
}

fn try_load_current_config(id: &str) -> Result<Config, ClientError> {
    // try to load the config as is
    if let Ok(cfg) = Config::read_from_default_path(id) {
        return if !cfg.validate() {
            Err(ClientError::ConfigValidationFailure)
        } else {
            Ok(cfg)
        };
    }

    // we couldn't load it - try upgrading it from older revisions
    try_upgrade_config(id)?;

    let config = match Config::read_from_default_path(id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!("Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})");
            return Err(ClientError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(ClientError::ConfigValidationFailure);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
