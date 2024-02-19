// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::config::old_config_v1_1_20_2::ConfigV1_1_20_2;
use crate::{
    config::{BaseClientConfig, Config},
    error::NetworkRequesterError,
};
use clap::{CommandFactory, Parser, Subcommand};
use log::{error, info, trace};
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_bin_common::version_checker;
use nym_client_core::client::base_client::storage::gateway_details::{
    OnDiskGatewayDetails, PersistedGatewayDetails,
};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::config::GatewayEndpointConfig;
use nym_client_core::error::ClientCoreError;
use nym_config::OptionalSet;
use std::sync::OnceLock;

mod build_info;
mod init;
mod run;
mod sign;

fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

#[derive(Parser)]
#[command(author = "Nymtech", version, about, long_version = pretty_build_info_static())]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[arg(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Flag used for disabling the printed banner in tty.
    #[arg(long)]
    pub(crate) no_banner: bool,

    #[command(subcommand)]
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
    enable_exit_policy: Option<bool>,

    open_proxy: Option<bool>,
    enable_statistics: Option<bool>,
    statistics_recipient: Option<String>,
}

// NOTE: make sure this is in sync with `gateway/src/commands/helpers.rs::override_network_requester_config`
pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    // as of 12.09.23 the below is true (not sure how this comment will rot in the future)
    // medium_toggle:
    // - sets secondary packet size to 16kb
    // - disables poisson distribution of the main traffic stream
    // - sets the cover traffic stream to 1 packet / 5s (on average)
    // - disables per hop delay
    //
    // fastmode (to be renamed to `fast-poisson`):
    // - sets average per hop delay to 10ms
    // - sets the cover traffic stream to 1 packet / 2000s (on average); for all intents and purposes it disables the stream
    // - sets the poisson distribution of the main traffic stream to 4ms, i.e. 250 packets / s on average
    //
    // no_cover:
    // - disables poisson distribution of the main traffic stream
    // - disables the secondary cover traffic stream

    // disable poisson rate in the BASE client if the NR option is enabled
    if config.network_requester.disable_poisson_rate {
        config.set_no_poisson_process();
    }

    // those should be enforced by `clap` when parsing the arguments
    if args.medium_toggle {
        assert!(!args.fastmode);
        assert!(!args.no_cover);

        config.set_medium_toggle();
    }

    config
        .with_base(
            BaseClientConfig::with_high_default_traffic_volume,
            args.fastmode,
        )
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
        .with_optional(Config::with_open_proxy, args.open_proxy)
        .with_optional(
            Config::with_old_allow_list,
            args.enable_exit_policy.map(|e| !e),
        )
        .with_optional(Config::with_enabled_statistics, args.enable_statistics)
        .with_optional(Config::with_statistics_recipient, args.statistics_recipient)
}

pub(crate) async fn execute(args: Cli) -> Result<(), NetworkRequesterError> {
    let bin_name = "nym-network-requester";

    match args.command {
        Commands::Init(m) => init::execute(m).await?,
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
    let persisted_details = PersistedGatewayDetails::new(details.into(), Some(&shared_keys))?;
    details_store
        .store_to_disk(&persisted_details)
        .map_err(|source| {
            NetworkRequesterError::ClientCoreError(ClientCoreError::GatewayDetailsStoreError {
                source: Box::new(source),
            })
        })
}

fn try_upgrade_v1_1_13_config(id: &str) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.13 config");
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

fn try_upgrade_v1_1_20_config(id: &str) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.20 config");
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

fn try_upgrade_v1_1_20_2_config(id: &str) -> Result<bool, NetworkRequesterError> {
    trace!("Trying to load as v1.1.20_2 config");

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

fn try_upgrade_config(id: &str) -> Result<(), NetworkRequesterError> {
    trace!("Attempting to upgrade config");
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
