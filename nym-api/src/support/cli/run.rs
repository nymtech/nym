// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::nyxd;

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

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Id of the nym-api we want to run
    #[clap(long)]
    // ugh. we had to make it optional in case somebody wanted to run `build-info`
    pub(crate) id: Option<String>,

    /// Specifies whether network monitoring is enabled on this API
    #[clap(short = 'm', long)]
    pub(crate) enable_monitor: Option<bool>,

    /// Specifies whether network rewarding is enabled on this API
    #[clap(short = 'r', long, requires = "enable_monitor", requires = "mnemonic")]
    pub(crate) enable_rewarding: Option<bool>,

    /// Specifies whether ephemera is used to aggregate monitor data on this API
    #[clap(short = 'e', long, requires = "enable_monitor")]
    pub(crate) enable_ephemera: Option<bool>,

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

    /// Ephemera configuration arguments.
    #[command(flatten)]
    pub(crate) ephemera_args: ephemera::cli::init::Cmd,
}
