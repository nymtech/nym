// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::config::Config;
use crate::support::config::default_config_filepath;
use crate::support::config::helpers::{initialise_new, try_load_current_config};
use ::nym_config::defaults::var_names::{MIXNET_CONTRACT_ADDRESS, VESTING_CONTRACT_ADDRESS};
use anyhow::Result;
use clap::Parser;
use lazy_static::lazy_static;
use nym_bin_common::bin_info;
use nym_config::defaults::var_names::NYXD;
use nym_config::OptionalSet;
use nym_validator_client::nyxd;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String = bin_info!().pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

// explicitly defined custom parser (as opposed to just using
// #[arg(value_parser = clap::value_parser!(u8).range(0..100))]
// for better error message
fn threshold_in_range(s: &str) -> Result<u8, String> {
    let threshold: usize = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a valid threshold number"))?;
    if threshold > 100 {
        Err(format!("{threshold} is not within the range 0-100"))
    } else {
        Ok(threshold as u8)
    }
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct CliArgs {
    /// Path pointing to an env file that configures the Nym API.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    /// Id of the nym-api we want to run
    #[clap(long)]
    pub(crate) id: String,

    /// Specifies whether network monitoring is enabled on this API
    #[clap(short = 'm', long)]
    pub(crate) enable_monitor: Option<bool>,

    /// Specifies whether network rewarding is enabled on this API
    #[clap(short = 'r', long, requires = "enable_monitor", requires = "mnemonic")]
    pub(crate) enable_rewarding: Option<bool>,

    /// Endpoint to nyxd instance from which the monitor will grab nodes to test
    #[clap(long)]
    pub(crate) nyxd_validator: Option<url::Url>,

    /// Address of the mixnet contract managing the network
    #[clap(long)]
    pub(crate) mixnet_contract: Option<nyxd::AccountId>,

    /// Address of the vesting contract holding locked tokens
    #[clap(long)]
    pub(crate) vesting_contract: Option<nyxd::AccountId>,

    /// Mnemonic of the network monitor used for rewarding operators
    // even though we're currently converting the mnemonic to string (and then back to the concrete type)
    // at least we're getting immediate validation when passing the arguments
    #[clap(long)]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    /// Specifies whether a config file based on provided arguments should be saved to a file
    #[clap(short = 'w', long)]
    pub(crate) save_config: bool,

    /// Specifies the minimum percentage of monitor test run data present in order to distribute rewards for given interval.
    #[clap(long, value_parser = threshold_in_range)]
    pub(crate) monitor_threshold: Option<u8>,

    /// Mixnodes with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    #[clap(long, value_parser = threshold_in_range)]
    pub(crate) min_mixnode_reliability: Option<u8>,

    /// Gateways with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    #[clap(long, value_parser = threshold_in_range)]
    pub(crate) min_gateway_reliability: Option<u8>,

    /// Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
    #[clap(long)]
    pub(crate) enabled_credentials_mode: Option<bool>,

    /// Announced address where coconut clients will connect.
    #[clap(long, hide = true)]
    pub(crate) announce_address: Option<url::Url>,

    /// Flag to indicate whether coconut signer authority is enabled on this API
    #[clap(
        long,
        requires = "mnemonic",
        requires = "announce_address",
        hide = true
    )]
    pub(crate) enable_coconut: Option<bool>,
}

pub(crate) fn override_config(config: Config, args: CliArgs) -> Config {
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
        .with_optional(
            Config::with_disabled_credentials_mode,
            args.enabled_credentials_mode.map(|b| !b),
        )
        .with_optional(Config::with_announce_address, args.announce_address)
        .with_optional(Config::with_coconut_signer_enabled, args.enable_coconut)
}

pub(crate) fn build_config(args: CliArgs) -> Result<Config> {
    let id = args.id.clone();

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
