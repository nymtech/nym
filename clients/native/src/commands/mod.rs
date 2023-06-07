// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::old_config_v1_1_13::OldConfigV1_1_13;
use crate::client::config::old_config_v1_1_19::ConfigV1_1_19;
use crate::client::config::{BaseClientConfig, Config};
use crate::error::ClientError;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use log::{error, info};
use nym_bin_common::build_information::BinaryBuildInformation;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_config::OptionalSet;
use std::error::Error;
use std::net::IpAddr;

pub(crate) mod init;
pub(crate) mod run;
pub(crate) mod upgrade;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String =
        BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the client.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialise a Nym client. Do this first!
    Init(init::Init),
    /// Run the Nym client with provided configuration client optionally overriding set parameters
    Run(run::Run),
    /// Try to upgrade the client
    Upgrade(upgrade::Upgrade),

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

pub(crate) async fn execute(args: &Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bin_name = "nym-native-client";

    match &args.command {
        Commands::Init(m) => init::execute(m).await?,
        Commands::Run(m) => run::execute(m).await?,
        Commands::Upgrade(m) => upgrade::execute(m),
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

    let updated_step1: ConfigV1_1_19 = old_config.into();
    let updated: Config = updated_step1.into();

    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_v1_1_19_config(id: &str) -> Result<bool, ClientError> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.19 (which is incompatible with the current one, i.e. +1.1.20)
    let Ok(old_config) = ConfigV1_1_19::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(false);
    };
    info!("It seems the client is using <= v1.1.19 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    updated.save_to_default_location()?;
    Ok(true)
}

fn try_upgrade_config(id: &str) -> Result<(), ClientError> {
    let upgraded = try_upgrade_v1_1_13_config(id)?;
    if !upgraded {
        try_upgrade_v1_1_19_config(id)?;
    }

    Ok(())
}

fn try_load_current_config(id: &str) -> Result<Config, ClientError> {
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
