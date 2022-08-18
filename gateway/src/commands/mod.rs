// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{process, str::FromStr};

use crate::{config::Config, Cli};
use clap::Subcommand;
use colored::Colorize;
use config::parse_validators;
use crypto::bech32_address_validation;
use network_defaults::var_names::{
    API_VALIDATOR, BECH32_PREFIX, CONFIGURED, NYMD_VALIDATOR, STATISTICS_SERVICE_DOMAIN_ADDRESS,
};

pub(crate) mod init;
pub(crate) mod node_details;
pub(crate) mod run;
pub(crate) mod sign;
pub(crate) mod upgrade;

#[cfg(all(not(feature = "eth"), not(feature = "coconut")))]
const DEFAULT_ETH_ENDPOINT: &str = "https://rinkeby.infura.io/v3/00000000000000000000000000000000";
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
}

// Configuration that can be overridden.
pub(crate) struct OverrideConfig {
    host: Option<String>,
    wallet_address: Option<String>,
    mix_port: Option<u16>,
    clients_port: Option<u16>,
    datastore: Option<String>,
    announce_host: Option<String>,
    enabled_statistics: Option<bool>,
    statistics_service_url: Option<String>,
    validator_apis: Option<String>,
    validators: Option<String>,
    mnemonic: Option<String>,

    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    enabled_credentials_mode: Option<bool>,

    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    eth_endpoint: Option<String>,
}

pub(crate) async fn execute(args: Cli) {
    match &args.command {
        Commands::Init(m) => init::execute(m).await,
        Commands::NodeDetails(m) => node_details::execute(m).await,
        Commands::Run(m) => run::execute(m).await,
        Commands::Sign(m) => sign::execute(m),
        Commands::Upgrade(m) => upgrade::execute(m).await,
    }
}

pub(crate) fn override_config(mut config: Config, args: OverrideConfig) -> Config {
    let mut was_host_overridden = false;
    if let Some(host) = args.host {
        config = config.with_listening_address(host);
        was_host_overridden = true;
    }

    if let Some(mix_port) = args.mix_port {
        config = config.with_mix_port(mix_port);
    }

    if let Some(clients_port) = args.clients_port {
        config = config.with_clients_port(clients_port);
    }

    if let Some(announce_host) = args.announce_host {
        config = config.with_announce_address(announce_host);
    } else if was_host_overridden {
        // make sure our 'mix-announce-host' always defaults to 'mix-host'
        config = config.announce_host_from_listening_host();
    }

    if let Some(enabled_statistics) = args.enabled_statistics {
        config = config.with_enabled_statistics(enabled_statistics);
    }

    if let Some(raw_url) = args.statistics_service_url {
        config = config.with_custom_statistics_service_url(
            raw_url
                .parse()
                .expect("the provided statistics service url is invalid!"),
        );
    } else if std::env::var(CONFIGURED).is_ok() {
        let raw_url = std::env::var(STATISTICS_SERVICE_DOMAIN_ADDRESS)
            .expect("statistics service url not set");
        config = config.with_custom_statistics_service_url(
            raw_url
                .parse()
                .expect("the provided statistics service url is invalid"),
        )
    }

    if let Some(raw_validators) = args.validator_apis {
        config = config.with_custom_validator_apis(parse_validators(&raw_validators));
    } else if std::env::var(CONFIGURED).is_ok() {
        let raw_validators = std::env::var(API_VALIDATOR).expect("api validator not set");
        config = config.with_custom_validator_apis(parse_validators(&raw_validators))
    }

    if let Some(raw_validators) = args.validators {
        config = config.with_custom_validator_nymd(parse_validators(&raw_validators));
    } else if std::env::var(CONFIGURED).is_ok() {
        let raw_validators = std::env::var(NYMD_VALIDATOR).expect("nymd validator not set");
        config = config.with_custom_validator_nymd(parse_validators(&raw_validators))
    }

    if let Some(wallet_address) = args.wallet_address {
        let trimmed = wallet_address.trim();
        validate_bech32_address_or_exit(trimmed);
        config = config.with_wallet_address(trimmed);
    }

    if let Some(datastore_path) = args.datastore {
        config = config.with_custom_persistent_store(datastore_path);
    }

    if let Some(cosmos_mnemonic) = args.mnemonic {
        config = config.with_cosmos_mnemonic(
            bip39::Mnemonic::from_str(&cosmos_mnemonic).expect("Provided mnemonic is invalid"),
        );
    }

    #[cfg(all(not(feature = "eth"), not(feature = "coconut")))]
    {
        config = config.with_eth_endpoint(String::from(DEFAULT_ETH_ENDPOINT));
    }

    // We set the disabled credentials mode flag if we either compile without 'eth', or if there is a flag we
    // can read from, which is when we build with 'eth' (and without 'coconut').
    if cfg!(not(feature = "eth")) {
        config = config.with_disabled_credentials_mode(true);
    }

    #[cfg(all(feature = "eth", not(feature = "coconut")))]
    {
        if let Some(enabled_credentials_mode) = args.enabled_credentials_mode {
            config = config.with_disabled_credentials_mode(enabled_credentials_mode);
        }

        if let Some(raw_validators) = args.validators {
            config = config.with_custom_validator_nymd(parse_validators(&raw_validators));
        }

        if let Some(eth_endpoint) = args.eth_endpoint {
            config = config.with_eth_endpoint(eth_endpoint);
        }
    }

    config
}

/// Ensures that a given bech32 address is valid, or exits
pub(crate) fn validate_bech32_address_or_exit(address: &str) {
    let prefix = std::env::var(BECH32_PREFIX).expect("bech32 prefix not set");
    if let Err(bech32_address_validation::Bech32Error::DecodeFailed(err)) =
        bech32_address_validation::try_bech32_decode(address)
    {
        let error_message = format!("Error: wallet address decoding failed: {}", err).red();
        println!("{}", error_message);
        println!("Exiting...");
        process::exit(1);
    }

    if let Err(bech32_address_validation::Bech32Error::WrongPrefix(err)) =
        bech32_address_validation::validate_bech32_prefix(&prefix, address)
    {
        let error_message = format!("Error: wallet address type is wrong, {}", err).red();
        println!("{}", error_message);
        println!("Exiting...");
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
