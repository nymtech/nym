// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::client::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::client::config::old_config_v1_1_20_2::ConfigV1_1_20_2;
use crate::client::config::old_config_v1_1_33::ConfigV1_1_33;
use crate::client::config::{BaseClientConfig, Config};
use crate::error::ClientError;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use log::{error, info};
use nym_bin_common::bin_info;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_client_core::cli_helpers::CliClient;
use nym_client_core::client::base_client::storage::migration_helpers::v1_1_33;
use nym_config::OptionalSet;
use std::error::Error;
use std::net::IpAddr;
use std::sync::OnceLock;

pub(crate) mod build_info;
pub(crate) mod import_credential;
pub(crate) mod init;
mod list_gateways;
pub(crate) mod run;

pub(crate) struct CliNativeClient;

impl CliClient for CliNativeClient {
    const NAME: &'static str = "native";
    type Error = ClientError;
    type Config = Config;

    async fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error> {
        try_upgrade_config(id).await
    }

    async fn try_load_current_config(id: &str) -> Result<Self::Config, Self::Error> {
        try_load_current_config(id).await
    }
}

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

    /// Import a pre-generated credential
    ImportCredential(import_credential::Args),

    /// List all registered with gateways
    ListGateways(list_gateways::Args),

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
        Commands::ImportCredential(m) => import_credential::execute(m).await?,
        Commands::ListGateways(args) => list_gateways::execute(args).await?,
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

async fn try_upgrade_v1_1_13_config(id: &str) -> Result<bool, ClientError> {
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
    let (updated_step3, gateway_config) = updated_step2.upgrade()?;
    let old_paths = updated_step3.storage_paths.clone();
    let updated = updated_step3.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_20_config(id: &str) -> Result<bool, ClientError> {
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
    let (updated_step2, gateway_config) = updated_step1.upgrade()?;
    let old_paths = updated_step2.storage_paths.clone();
    let updated = updated_step2.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;
    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_20_2_config(id: &str) -> Result<bool, ClientError> {
    // explicitly load it as v1.1.20_2 (which is incompatible with the current one, i.e. +1.1.21)
    let Ok(old_config) = ConfigV1_1_20_2::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.20_2 config template.");
    info!("It is going to get updated to the current specification.");

    let (updated_step1, gateway_config) = old_config.upgrade()?;
    let old_paths = updated_step1.storage_paths.clone();
    let updated = updated_step1.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        Some(gateway_config),
    )
    .await?;
    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_v1_1_33_config(id: &str) -> Result<bool, ClientError> {
    // explicitly load it as v1.1.33 (which is incompatible with the current one, i.e. +1.1.34)
    let Ok(old_config) = ConfigV1_1_33::read_from_default_path(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.33 config template.");
    info!("It is going to get updated to the current specification.");

    let old_paths = old_config.storage_paths.clone();
    let updated = old_config.try_upgrade()?;

    v1_1_33::migrate_gateway_details(
        &old_paths.common_paths,
        &updated.storage_paths.common_paths,
        None,
    )
    .await?;

    updated.save_to_default_location()?;
    Ok(true)
}

async fn try_upgrade_config(id: &str) -> Result<(), ClientError> {
    if try_upgrade_v1_1_13_config(id).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_config(id).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_20_2_config(id).await? {
        return Ok(());
    }
    if try_upgrade_v1_1_33_config(id).await? {
        return Ok(());
    }

    Ok(())
}

async fn try_load_current_config(id: &str) -> Result<Config, ClientError> {
    // try to load the config as is
    if let Ok(cfg) = Config::read_from_default_path(id) {
        return if !cfg.validate() {
            Err(ClientError::ConfigValidationFailure)
        } else {
            Ok(cfg)
        };
    }

    // we couldn't load it - try upgrading it from older revisions
    try_upgrade_config(id).await?;

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
