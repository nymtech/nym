// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::config::Config;
use crate::support::config::default_config_filepath;
use crate::support::config::helpers::{initialise_new, try_load_current_config};
use ::nym_config::defaults::var_names::{MIXNET_CONTRACT_ADDRESS, VESTING_CONTRACT_ADDRESS};
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use lazy_static::lazy_static;
use nym_bin_common::bin_info;
use nym_config::defaults::var_names::NYXD;
use nym_config::OptionalSet;

pub(crate) mod build_info;
pub(crate) mod run;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

#[derive(Parser, Debug)]
#[command(args_conflicts_with_subcommands = true)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Cli {
    /// Path pointing to an env file that configures the Nym API.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    pub(crate) command: Option<Commands>,

    // this shouldn't really be here, but we don't want to break backwards compat
    #[clap(flatten)]
    pub(crate) run: run::Args,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Run the Nym Api with provided configuration optionally overriding set parameters
    Run(Box<run::Args>),

    /// Show build information of this binary
    BuildInfo(build_info::Args),
}

pub(crate) fn override_config(config: Config, args: run::Args) -> Config {
    config
        .with_optional_env(
            Config::with_custom_nyxd_validator,
            args.nyxd_validator,
            NYXD,
        )
        .with_optional_env(
            Config::with_custom_mixnet_contract,
            args.mixnet_contract,
            MIXNET_CONTRACT_ADDRESS,
        )
        .with_optional_env(
            Config::with_custom_vesting_contract,
            args.vesting_contract,
            VESTING_CONTRACT_ADDRESS,
        )
        .with_optional(Config::with_mnemonic, args.mnemonic)
        .with_optional(
            Config::with_minimum_interval_monitor_threshold,
            args.monitor_threshold,
        )
        .with_optional(
            Config::with_min_mixnode_reliability,
            args.min_mixnode_reliability,
        )
        .with_optional(
            Config::with_min_gateway_reliability,
            args.min_gateway_reliability,
        )
        .with_optional(Config::with_network_monitor_enabled, args.enable_monitor)
        .with_optional(Config::with_rewarding_enabled, args.enable_rewarding)
        .with_optional(Config::with_ephemera_enabled, args.enable_ephemera)
        .with_optional(
            Config::with_disabled_credentials_mode,
            args.enabled_credentials_mode.map(|b| !b),
        )
        .with_optional(Config::with_announce_address, args.announce_address)
        .with_optional(Config::with_coconut_signer_enabled, args.enable_coconut)
        .with_optional(Config::with_ephemera_ip, args.ephemera_args.ephemera_ip)
        .with_optional(
            Config::with_ephemera_protocol_port,
            args.ephemera_args.ephemera_protocol_port,
        )
        .with_optional(
            Config::with_ephemera_websocket_port,
            args.ephemera_args.ephemera_websocket_port,
        )
        .with_optional(
            Config::with_ephemera_http_api_port,
            args.ephemera_args.ephemera_http_api_port,
        )
}

pub(crate) fn build_config(args: run::Args) -> Result<Config> {
    let id = match &args.id {
        Some(id) => id.clone(),
        None => {
            error!("--id argument must be provided to run nym-api");
            bail!("--id argument must be provided to run nym-api")
        }
    };

    // try to load config from the file, if it doesn't exist, use default values
    let config = match try_load_current_config(&id) {
        Ok(cfg) => cfg,
        Err(err) => {
            let config_path = default_config_filepath(&id);
            warn!(
                "Could not load the configuration file from {}: {err}. Either the file did not exist or was malformed. Using the default values instead",
                config_path.display()
            );

            initialise_new(&id)?
        }
    };

    let config = override_config(config, args);

    Ok(config)
}
