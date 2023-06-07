// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_20::ConfigV1_1_20;
use crate::{config::Config, Cli};
use anyhow::anyhow;
use clap::CommandFactory;
use clap::Subcommand;
use colored::Colorize;
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_bin_common::version_checker;
use nym_config::defaults::var_names::{BECH32_PREFIX, NYM_API};
use nym_config::OptionalSet;
use nym_crypto::bech32_address_validation;
use std::net::IpAddr;
use std::process;

mod describe;
mod init;
mod node_details;
mod run;
mod sign;
mod upgrade;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Describe your mixnode and tell people why they should delegate state to you
    Describe(describe::Describe),

    /// Initialise the mixnode
    Init(init::Init),

    /// Starts the mixnode
    Run(run::Run),

    /// Sign text to prove ownership of this mixnode
    Sign(sign::Sign),

    /// Try to upgrade the mixnode
    Upgrade(upgrade::Upgrade),

    /// Show details of this mixnode
    NodeDetails(node_details::NodeDetails),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

// Configuration that can be overridden.
struct OverrideConfig {
    id: String,
    host: Option<IpAddr>,
    mix_port: Option<u16>,
    verloc_port: Option<u16>,
    http_api_port: Option<u16>,
    nym_apis: Option<Vec<url::Url>>,
}

pub(crate) async fn execute(args: Cli) -> anyhow::Result<()> {
    let bin_name = "nym-mixnode";

    match args.command {
        Commands::Describe(m) => describe::execute(m)?,
        Commands::Init(m) => init::execute(&m),
        Commands::Run(m) => run::execute(&m).await?,
        Commands::Sign(m) => sign::execute(&m)?,
        Commands::Upgrade(m) => upgrade::execute(&m)?,
        Commands::NodeDetails(m) => node_details::execute(&m)?,
        Commands::Completions(s) => s.generate(&mut crate::Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }
    Ok(())
}

fn override_config(config: Config, args: OverrideConfig) -> Config {
    config
        .with_optional(Config::with_listening_address, args.host)
        .with_optional(Config::with_mix_port, args.mix_port)
        .with_optional(Config::with_verloc_port, args.verloc_port)
        .with_optional(Config::with_http_api_port, args.http_api_port)
        .with_optional_custom_env(
            Config::with_custom_nym_apis,
            args.nym_apis,
            NYM_API,
            nym_config::parse_urls,
        )
}

/// Ensures that a given bech32 address is valid, or exits
pub(crate) fn validate_bech32_address_or_exit(address: &str) {
    let prefix = std::env::var(BECH32_PREFIX).expect("bech32 prefix not set");
    if let Err(bech32_address_validation::Bech32Error::DecodeFailed(err)) =
        bech32_address_validation::try_bech32_decode(address)
    {
        let error_message = format!("Error: wallet address decoding failed: {err}").red();
        error!("{}", error_message);
        error!("Exiting...");
        process::exit(1);
    }

    if let Err(bech32_address_validation::Bech32Error::WrongPrefix(err)) =
        bech32_address_validation::validate_bech32_prefix(&prefix, address)
    {
        let error_message = format!("Error: wallet address type is wrong, {err}").red();
        error!("{}", error_message);
        error!("Exiting...");
        process::exit(1);
    }
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
pub(crate) fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = &cfg.mixnode.version;
    if binary_version == config_version {
        true
    } else {
        warn!("The mixnode binary has different version than what is specified in config file! {} and {}", binary_version, config_version);
        if version_checker::is_minor_version_compatible(binary_version, config_version) {
            info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    }
}

fn try_upgrade_v1_1_20_config(id: &str) -> std::io::Result<()> {
    use nym_config::legacy_helpers::nym_config::MigrationNymConfig;

    // explicitly load it as v1.1.20 (which is incompatible with the current, i.e. 1.1.21+)
    let Ok(old_config) = ConfigV1_1_20::load_from_file(id) else {
        // if we failed to load it, there might have been nothing to upgrade
        // or maybe it was an even older file. in either way. just ignore it and carry on with our day
        return Ok(());
    };
    info!("It seems the mixnode is using <= v1.1.20 config template.");
    info!("It is going to get updated to the current specification.");

    let updated: Config = old_config.into();
    updated.save_to_default_location()
}

fn try_load_current_config(id: &str) -> anyhow::Result<Config> {
    try_upgrade_v1_1_20_config(id)?;

    Config::read_from_default_path(id).map_err(|err| {
        let error_msg =
            format!(
                "Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})",
            );
        error!("{error_msg}");
        anyhow!(error_msg)
    })
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
