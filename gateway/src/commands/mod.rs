// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{config::Config, Cli};
use clap::CommandFactory;
use clap::Subcommand;
use colored::Colorize;
use completions::{fig_generate, ArgShell};
use config::OptionalSet;
use crypto::bech32_address_validation;
use network_defaults::var_names::{BECH32_PREFIX, NYM_API, STATISTICS_SERVICE_DOMAIN_ADDRESS};
use std::net::IpAddr;
use std::path::PathBuf;
use std::process;
use validator_client::nyxd::{self};

pub(crate) mod init;
pub(crate) mod node_details;
pub(crate) mod run;
pub(crate) mod sign;
pub(crate) mod upgrade;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialise the gateway
    Init(init::Init),

    /// Show details of this gateway
    NodeDetails(node_details::NodeDetails),

    /// Starts the gateway
    Run(run::Run),

    /// Sign text to prove ownership of this mixnode
    Sign(sign::Sign),

    /// Try to upgrade the gateway
    Upgrade(upgrade::Upgrade),

    /// Generate shell completions
    Completions(ArgShell),

    /// Generate Fig specification
    GenerateFigSpec,
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    host: Option<IpAddr>,
    wallet_address: Option<nyxd::AccountId>,
    mix_port: Option<u16>,
    clients_port: Option<u16>,
    datastore: Option<PathBuf>,
    announce_host: Option<String>,
    enabled_statistics: Option<bool>,
    statistics_service_url: Option<url::Url>,
    nym_apis: Option<Vec<url::Url>>,
    mnemonic: Option<bip39::Mnemonic>,

    #[cfg(feature = "coconut")]
    nyxd_urls: Option<Vec<url::Url>>,
    #[cfg(feature = "coconut")]
    only_coconut_credentials: Option<bool>,
}

pub(crate) async fn execute(args: Cli) {
    let bin_name = "nym-gateway";

    let output = args.output();

    match &args.command {
        Commands::Init(m) => init::execute(m, output.clone()).await,
        Commands::NodeDetails(m) => node_details::execute(m, output.clone()).await,
        Commands::Run(m) => run::execute(m, output.clone()).await,
        Commands::Sign(m) => sign::execute(m),
        Commands::Upgrade(m) => upgrade::execute(m).await,
        Commands::Completions(s) => s.generate(&mut crate::Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }
}

pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    // special case that I'm not sure could be easily handled with the trait
    let mut was_host_overridden = false;
    if let Some(host) = args.host {
        config = config.with_listening_address(host);
        was_host_overridden = true;
    }

    if let Some(announce_host) = args.announce_host {
        config = config.with_announce_address(announce_host);
    } else if was_host_overridden {
        // make sure our 'mix-announce-host' always defaults to 'mix-host'
        config = config.announce_host_from_listening_host();
    }

    config = config
        .with_optional(Config::with_mix_port, args.mix_port)
        .with_optional(Config::with_clients_port, args.clients_port)
        .with_optional_custom_env(
            Config::with_custom_nym_apis,
            args.nym_apis,
            NYM_API,
            config::parse_urls,
        )
        .with_optional(Config::with_enabled_statistics, args.enabled_statistics)
        .with_optional_env(
            Config::with_custom_statistics_service_url,
            args.statistics_service_url,
            STATISTICS_SERVICE_DOMAIN_ADDRESS,
        )
        .with_optional(
            |cfg, wallet_address| {
                validate_bech32_address_or_exit(wallet_address.as_ref());
                cfg.with_wallet_address(wallet_address)
            },
            args.wallet_address,
        )
        .with_optional(Config::with_custom_persistent_store, args.datastore)
        .with_optional(Config::with_cosmos_mnemonic, args.mnemonic);

    #[cfg(feature = "coconut")]
    {
        use network_defaults::var_names::NYXD;

        config = config
            .with_optional_custom_env(
                Config::with_custom_validator_nyxd,
                args.nyxd_urls,
                NYXD,
                config::parse_urls,
            )
            .with_optional(
                Config::with_only_coconut_credentials,
                args.only_coconut_credentials,
            );
    }

    config
}

/// Ensures that a given bech32 address is valid, or exits
pub(crate) fn validate_bech32_address_or_exit(address: &str) {
    let prefix = std::env::var(BECH32_PREFIX).expect("bech32 prefix not set");
    if let Err(bech32_address_validation::Bech32Error::DecodeFailed(err)) =
        bech32_address_validation::try_bech32_decode(address)
    {
        let error_message = format!("Error: wallet address decoding failed: {err}").red();
        eprintln!("{}", error_message);
        eprintln!("Exiting...");
        process::exit(1);
    }

    if let Err(bech32_address_validation::Bech32Error::WrongPrefix(err)) =
        bech32_address_validation::validate_bech32_prefix(&prefix, address)
    {
        let error_message = format!("Error: wallet address type is wrong, {err}").red();
        eprintln!("{}", error_message);
        eprintln!("Exiting...");
        process::exit(1);
    }
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
pub(crate) fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = cfg.get_version();
    if binary_version != config_version {
        log::warn!("The gateway binary has different version than what is specified in config file! {} and {}", binary_version, config_version);
        if version_checker::is_minor_version_compatible(binary_version, config_version) {
            log::info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            log::error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    } else {
        true
    }
}
