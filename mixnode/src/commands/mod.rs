// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::Cli;
use clap::CommandFactory;
use clap::Subcommand;
use colored::Colorize;
use log::{error, info, warn};
use nym_bin_common::completions::{fig_generate, ArgShell};
use nym_bin_common::version_checker;
use nym_config::defaults::var_names::{BECH32_PREFIX, NYM_API};
use nym_config::OptionalSet;
use nym_crypto::bech32_address_validation;
use nym_mixnode::config::{default_config_filepath, Config};
use nym_mixnode::error::MixnodeError;
use std::net::IpAddr;
use std::process;

mod build_info;
mod describe;
mod init;
mod node_details;
mod run;
mod sign;
mod upgrade_helpers;

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

    /// Show details of this mixnode
    NodeDetails(node_details::NodeDetails),

    /// Show build information of this binary
    BuildInfo(build_info::BuildInfo),

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
    metrics_key: Option<String>,
}

pub(crate) async fn execute(args: Cli) -> anyhow::Result<()> {
    let bin_name = "nym-mixnode";

    match args.command {
        Commands::Describe(m) => describe::execute(m)?,
        Commands::Init(m) => init::execute(&m)?,
        Commands::Run(m) => run::execute(&m).await?,
        Commands::Sign(m) => sign::execute(&m)?,
        Commands::NodeDetails(m) => node_details::execute(&m)?,
        Commands::BuildInfo(m) => build_info::execute(m),
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
        .with_optional(Config::with_metrics_key, args.metrics_key)
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

fn try_load_current_config(id: &str) -> Result<Config, MixnodeError> {
    upgrade_helpers::try_upgrade_config(id)?;

    Config::read_from_default_path(id).map_err(|err| {
        error!(
            "Failed to load config for {id}. Are you sure you have run `init` before? (Error was: {err})",
        );
        MixnodeError::ConfigLoadFailure {
            path: default_config_filepath(id),
            id: id.to_string(),
            source: err,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
