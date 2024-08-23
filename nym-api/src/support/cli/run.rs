// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::start_nym_api_tasks;
use crate::support::config::helpers::try_load_current_config;
use cfg_if::cfg_if;

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
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    // args take precedence over env
    let config = try_load_current_config(&args.id)?
        .override_with_env()
        .override_with_args(args);

    config.validate()?;

    #[cfg(feature = "axum")]
    let mut axum_shutdown = crate::v2::start_nym_api_tasks_v2(&config).await?;
    let mut rocket_shutdown = start_nym_api_tasks(config).await?;

    // it doesn't matter which server catches the interrupt: it needs only be caught once
    if let Err(err) = rocket_shutdown.task_manager_handle.catch_interrupt().await {
        error!("Error stopping Rocket tasks: {err}");
    }

    log::info!("Stopping nym API");
    rocket_shutdown.rocket_handle.notify();

    // because Rocket caught the interrupt, it had already signalled its
    // background tasks to retire. Now do that for axum
    cfg_if! {
        if #[cfg(feature = "axum")] {
            axum_shutdown.task_manager_mut().signal_shutdown().ok();
            axum_shutdown.task_manager_mut().wait_for_shutdown().await;

            let running_server = axum_shutdown.shutdown_axum();

            match running_server.await {
                Ok(Ok(_)) => {
                    info!("Axum HTTP server shut down without errors");
                },
                Ok(Err(err)) => {
                    error!("Axum HTTP server terminated with: {err}");
                    anyhow::bail!(err)
                },
                Err(err) => {
                    error!("Server task panicked: {err}");
                }
            };
        }
    }

    Ok(())
}
