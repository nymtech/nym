// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::config::old_config_v1_1_20_2::ConfigV1_1_20_2;
use crate::{
    config::{BaseClientConfig, Config},
    error::NetworkRequesterError,
};
use clap::{CommandFactory, Parser, Subcommand};
use log::{error, info};
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_bin_common::version_checker;
use nym_client_core::client::base_client::storage::gateway_details::{
    OnDiskGatewayDetails, PersistedGatewayDetails,
};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::config::GatewayEndpointConfig;
use nym_client_core::error::ClientCoreError;
use nym_sphinx::params::PacketSize;

mod build_info;
mod init;
mod run;
mod sign;

lazy_static::lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
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

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialize a network-requester. Do this first!
    Init(init::Init),

    /// Run the network requester with the provided configuration and optionally override
    /// parameters.
    Run(run::Run),

    /// Sign to prove ownership of this network requester
    Sign(sign::Sign),

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
    fastmode: bool,
    no_cover: bool,
    medium_toggle: bool,
    nyxd_urls: Option<Vec<url::Url>>,
    enabled_credentials_mode: Option<bool>,
}

pub(crate) fn override_config(config: Config, args: OverrideConfig) -> Config {
    let disable_cover_traffic_with_keepalive = args.medium_toggle;
    let secondary_packet_size = args.medium_toggle.then_some(PacketSize::ExtendedPacket16);
    let no_per_hop_delays = args.medium_toggle;

    config
        .with_base(
            BaseClientConfig::with_high_default_traffic_volume,
            args.fastmode,
        )
        .with_base(
            // NOTE: This interacts with disabling cover traffic fully, so we want to this to be set before
            BaseClientConfig::with_disabled_cover_traffic_with_keepalive,
            disable_cover_traffic_with_keepalive,
        )
        .with_base(
            BaseClientConfig::with_secondary_packet_size,
            secondary_packet_size,
        )
        .with_base(BaseClientConfig::with_no_per_hop_delays, no_per_hop_delays)
        // NOTE: see comment above about the order of the other disble cover traffic config
        .with_base(BaseClientConfig::with_disabled_cover_traffic, args.no_cover)
        .with_optional_base_custom_env(
            BaseClientConfig::with_custom_nym_apis,
            args.nym_apis,
            nym_network_defaults::var_names::NYM_API,
            nym_config::parse_urls,
        )
        .with_optional_base_custom_env(
            BaseClientConfig::with_custom_nyxd,
            args.nyxd_urls,
            nym_network_defaults::var_names::NYXD,
            nym_config::parse_urls,
        )
        .with_optional_base(
            BaseClientConfig::with_disabled_credentials,
            args.enabled_credentials_mode.map(|b| !b),
        )
}

pub(crate) async fn execute(args: Cli) -> Result<(), NetworkRequesterError> {
    let bin_name = "nym-network-requester";

    match args.command {
        Commands::Init(m) => init::execute(&m).await?,
        Commands::Run(m) => run::execute(&m).await?,
        Commands::Sign(m) => sign::execute(&m).await?,
        Commands::BuildInfo(m) => build_info::execute(m),
        Commands::Completions(s) => s.generate(&mut Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut Cli::command(), bin_name),
    }
    Ok(())
}

fn persist_gateway_details(
    config: &Config,
    details: GatewayEndpointConfig,
) -> Result<(), NetworkRequesterError> {
    let details_store =
        OnDiskGatewayDetails::new(&config.storage_paths.common_paths.gateway_details);
    let keys_store = OnDiskKeys::new(config.storage_paths.common_paths.keys.clone());
    let shared_keys = keys_store.ephemeral_load_gateway_keys().map_err(|source| {
        NetworkRequesterError::ClientCoreError(ClientCoreError::KeyStoreError {
            source: Box::new(source),
        })
    })?;
    let persisted_details = PersistedGatewayDetails::new(details, &shared_keys);
    details_store
        .store_to_disk(&persisted_details)
        .map_err(|source| {
            NetworkRequesterError::ClientCoreError(ClientCoreError::GatewayDetailsStoreError {
                source: Box::new(source),
            })
        })
}

fn try_upgrade_v1_1_13_config(id: &str) -> Result<bool, NetworkRequesterError> {
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
    let (updated, gateway_config) = updated_step2.upgrade();
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_v1_1_20_config(id: &str) -> Result<bool, NetworkRequesterError> {
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
    let (updated, gateway_config) = updated_step1.upgrade();
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_v1_1_20_2_config(id: &str) -> Result<bool, NetworkRequesterError> {
    // explicitly load it as v1.1.20_2 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20_2::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20_2 config template.");
    info!("It is going to get updated to the current specification.");

    let (updated, gateway_config) = old_config.upgrade();
    persist_gateway_details(&updated, gateway_config)?;

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_config(id: &str) -> Result<(), NetworkRequesterError> {
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

fn try_load_current_config(id: &str) -> Result<Config, NetworkRequesterError> {
    // try to load the config as is
    if let Ok(cfg) = Config::read_from_default_path(id) {
        return if !cfg.validate() {
            Err(NetworkRequesterError::ConfigValidationFailure)
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
            return Err(NetworkRequesterError::FailedToLoadConfig(id.to_string()));
        }
    };

    if !config.validate() {
        return Err(NetworkRequesterError::ConfigValidationFailure);
    }

    Ok(config)
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = &cfg.base.client.version;
    if binary_version == config_version {
        true
    } else {
        log::warn!(
            "The native-client binary has different version than what is specified \
            in config file! {binary_version} and {config_version}",
        );
        if version_checker::is_minor_version_compatible(binary_version, config_version) {
            log::info!(
                "but they are still semver compatible. \
                However, consider running the `upgrade` command"
            );
            true
        } else {
            log::error!(
                "and they are semver incompatible! - \
                please run the `upgrade` command before attempting `run` again"
            );
            false
        }
    }
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
