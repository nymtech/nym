// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{config::Config, Cli};
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
use validator_client::nyxd;

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
    wallet_address: Option<nyxd::AccountId>,
    mix_port: Option<u16>,
    verloc_port: Option<u16>,
    http_api_port: Option<u16>,
    announce_host: Option<String>,
    nym_apis: Option<Vec<url::Url>>,
}

pub(crate) async fn execute(args: Cli) {
    let bin_name = "nym-mixnode";

    match args.command {
        Commands::Describe(m) => describe::execute(m),
        Commands::Init(m) => init::execute(&m),
        Commands::Run(m) => run::execute(&m).await,
        Commands::Sign(m) => sign::execute(&m),
        Commands::Upgrade(m) => upgrade::execute(&m),
        Commands::NodeDetails(m) => node_details::execute(&m),
        Commands::Completions(s) => s.generate(&mut crate::Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }
}

fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    // special case that I'm not sure could be easily handled with the trait
    let mut was_host_overridden = false;
    if let Some(host) = args.host {
        config = config.with_listening_address(host);
        was_host_overridden = true;
    }

    if let Some(announce_host) = args.announce_host {
        config = config.with_announce_address(announce_host);
    } else if was_host_overridden {
        // make sure our 'announce-host' always defaults to 'host'
        config = config.announce_address_from_listening_address()
    }

    config
        .with_optional(Config::with_mix_port, args.mix_port)
        .with_optional(Config::with_verloc_port, args.verloc_port)
        .with_optional(Config::with_http_api_port, args.http_api_port)
        .with_optional_custom_env(
            Config::with_custom_nym_apis,
            args.nym_apis,
            NYM_API,
            nym_config::parse_urls,
        )
        .with_optional(
            |cfg, wallet_address| {
                validate_bech32_address_or_exit(wallet_address.as_ref());
                cfg.with_wallet_address(wallet_address)
            },
            args.wallet_address,
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
    let config_version = cfg.get_version();
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
