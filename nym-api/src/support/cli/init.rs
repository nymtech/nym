// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config::default_config_filepath;
use crate::support::config::helpers::initialise_new;
use anyhow::bail;
use std::net::SocketAddr;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Id of the nym-api we want to initialise. if unspecified, a default value will be used.
    /// default: "default"
    #[clap(long, default_value = "default")]
    pub(crate) id: String,

    /// Specifies whether network monitoring is enabled on this API
    /// default: false
    #[clap(short = 'm', long)]
    pub(crate) enable_monitor: bool,

    /// Specifies whether network rewarding is enabled on this API
    /// default: false
    #[clap(short = 'r', long, requires = "enable_monitor", requires = "mnemonic")]
    pub(crate) enable_rewarding: bool,

    /// Endpoint to nyxd instance used for contract information.
    /// default: http://localhost:26657
    #[clap(long)]
    pub(crate) nyxd_validator: Option<url::Url>,

    /// Mnemonic of the network monitor used for sending rewarding and zk-nyms transactions
    /// default: None
    #[clap(long)]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    /// Flag to indicate whether credential signer authority is enabled on this API
    /// default: false
    #[clap(
        long,
        requires = "mnemonic",
        requires = "announce_address",
        alias = "enable_coconut"
    )]
    pub(crate) enable_zk_nym: bool,

    /// Announced address that is going to be put in the DKG contract where zk-nym clients will connect
    /// to obtain their credentials
    /// default: None
    #[clap(long)]
    pub(crate) announce_address: Option<url::Url>,

    /// Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
    #[clap(long, requires = "enable_monitor")]
    pub(crate) monitor_credentials_mode: bool,

    /// Socket address this api will use for binding its http API.
    /// default: `127.0.0.1:8080` in `debug` builds and `0.0.0.0:8080` in `release`
    #[clap(long)]
    pub(crate) bind_address: Option<SocketAddr>,
    // #[clap(short, long, default_value_t = OutputFormat::default())]
    // output: OutputFormat,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    eprintln!("initialising nym-api...");

    // let output = args.output;

    let config_path = default_config_filepath(&args.id);
    if config_path.exists() {
        // don't bother with attempting to override some of the data and preserving the rest of it
        // if the config exists.
        // this situation should never occur under normal circumstances, so it's up to the user to deal with it
        bail!("there already exists a configuration file at '{}'. If you intend to replace it, you need to manually remove it first. Make sure to make backup of any keys and datastores first.", config_path.display())
    }

    let config = initialise_new(&args.id)?;
    // args take precedence over env
    config
        .override_with_env()
        .override_with_args(args)
        .try_save()?;

    Ok(())
}
