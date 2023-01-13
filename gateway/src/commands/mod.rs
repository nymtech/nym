// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::GatewayError;
use crate::{config::Config, Cli};
use clap::CommandFactory;
use clap::Subcommand;
use completions::{fig_generate, ArgShell};
use config::OptionalSet;
use network_defaults::var_names::{BECH32_PREFIX, NYM_API, STATISTICS_SERVICE_DOMAIN_ADDRESS};
use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use validator_client::nyxd::{self, AccountId};

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
#[derive(Default)]
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

pub(crate) async fn execute(args: Cli) -> Result<(), Box<dyn Error + Send + Sync>> {
    let bin_name = "nym-gateway";

    let output = args.output();

    match args.command {
        Commands::Init(m) => init::execute(m, output.clone()).await?,
        Commands::NodeDetails(m) => node_details::execute(m, output.clone()).await?,
        Commands::Run(m) => run::execute(m, output.clone()).await?,
        Commands::Sign(m) => sign::execute(m)?,
        Commands::Upgrade(m) => upgrade::execute(&m).await,
        Commands::Completions(s) => s.generate(&mut crate::Cli::command(), bin_name),
        Commands::GenerateFigSpec => fig_generate(&mut crate::Cli::command(), bin_name),
    }
    Ok(())
}

pub(crate) fn override_config(
    mut config: Config,
    args: OverrideConfig,
) -> Result<Config, GatewayError> {
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
        .with_validated_optional(
            |cfg, wallet_address| cfg.with_wallet_address(wallet_address),
            args.wallet_address,
            ensure_correct_bech32_prefix,
        )?
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

    Ok(config)
}

/// Ensures that a given bech32 address is valid
pub(crate) fn ensure_correct_bech32_prefix(address: &AccountId) -> Result<(), GatewayError> {
    let expected_prefix = std::env::var(BECH32_PREFIX).expect("bech32 prefix not set");
    let actual_prefix = address.prefix();
    if expected_prefix != actual_prefix {
        return Err(GatewayError::InvalidBech32AccountPrefix {
            account: address.to_owned(),
            expected_prefix,
            actual_prefix: actual_prefix.to_owned(),
        });
    }

    Ok(())
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
pub(crate) fn ensure_config_version_compatibility(cfg: &Config) -> Result<(), GatewayError> {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = cfg.get_version();
    if binary_version != config_version {
        log::warn!("The gateway binary has different version than what is specified in config file! {} and {}", binary_version, config_version);
        if version_checker::is_minor_version_compatible(binary_version, config_version) {
            log::info!("but they are still semver compatible. However, consider running the `upgrade` command");
            Ok(())
        } else {
            log::error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            Err(GatewayError::LocalVersionCheckFailure {
                binary_version: binary_version.to_owned(),
                config_version: config_version.to_owned(),
            })
        }
    } else {
        Ok(())
    }
}
