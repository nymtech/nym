// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::start_nym_api_tasks;
use crate::support::config::helpers::try_load_current_config;
use anyhow::bail;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Id of the nym-api we want to run.if unspecified, a default value will be used.
    /// default: "default"
    #[clap(long, default_value = "default")]
    pub(crate) id: String,

    /// Specifies whether network monitoring is enabled on this API
    /// default: None - config value will be used instead
    #[clap(short = 'm', long)]
    pub(crate) enable_monitor: Option<bool>,

    /// Specifies whether network rewarding is enabled on this API
    /// default: None - config value will be used instead
    #[clap(short = 'r', long, requires = "enable_monitor", requires = "mnemonic")]
    pub(crate) enable_rewarding: Option<bool>,

    /// Endpoint to nyxd instance used for contract information.
    /// default: None - config value will be used instead
    #[clap(long)]
    pub(crate) nyxd_validator: Option<url::Url>,

    /// Mnemonic of the network monitor used for sending rewarding and zk-nyms transactions
    /// default: None - config value will be used instead
    #[clap(long)]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    /// Flag to indicate whether coconut signer authority is enabled on this API
    /// default: None - config value will be used instead
    #[clap(
        long,
        requires = "mnemonic",
        requires = "announce_address",
        alias = "enable_coconut"
    )]
    pub(crate) enable_zk_nym: Option<bool>,

    /// Announced address that is going to be put in the DKG contract where zk-nym clients will connect
    /// to obtain their credentials    
    /// default: None - config value will be used instead
    #[clap(long)]
    pub(crate) announce_address: Option<url::Url>,

    /// Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
    /// default: None - config value will be used instead
    #[clap(long)]
    pub(crate) monitor_credentials_mode: Option<bool>,
    // /// Ephemera configuration arguments.
    // #[command(flatten)]
    // pub(crate) ephemera_args: ephemera::cli::init::Cmd,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    // args take precedence over env
    let config = try_load_current_config(&args.id)?
        .override_with_env()
        .override_with_args(args);

    config.validate()?;

    let mut shutdown_handlers = start_nym_api_tasks(config).await?;

    let res = shutdown_handlers
        .task_manager_handle
        .catch_interrupt()
        .await;
    log::info!("Stopping nym API");
    shutdown_handlers.rocket_handle.notify();

    if let Err(err) = res {
        // that's a nasty workaround, but anyhow errors are generally nicer, especially on exit
        bail!("{err}")
    }

    Ok(())
}
