// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::{try_load_current_config, version_check};
use crate::{
    cli::{override_config, OverrideConfig},
    error::NetworkRequesterError,
};
use clap::Args;
use log::error;
use nym_client_core::cli_helpers::client_run::CommonClientRunArgs;

const ENABLE_STATISTICS: &str = "enable-statistics";

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Clone)]
pub(crate) struct Run {
    #[command(flatten)]
    common_args: CommonClientRunArgs,

    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[arg(long)]
    open_proxy: Option<bool>,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[arg(long)]
    enable_statistics: Option<bool>,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym
    /// aggregator client
    #[arg(long)]
    statistics_recipient: Option<String>,

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[arg(
        long,
        hide = true,
        conflicts_with = "no_cover",
        conflicts_with = "fastmode"
    )]
    medium_toggle: bool,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nym_apis: None,
            fastmode: run_config.common_args.fastmode,
            no_cover: run_config.common_args.no_cover,
            medium_toggle: run_config.medium_toggle,
            nyxd_urls: run_config.common_args.nyxd_urls,
            enabled_credentials_mode: run_config.common_args.enabled_credentials_mode,
            open_proxy: run_config.open_proxy,
            enable_statistics: run_config.enable_statistics,
            statistics_recipient: run_config.statistics_recipient,
        }
    }
}

pub(crate) async fn execute(args: &Run) -> Result<(), NetworkRequesterError> {
    let mut config = try_load_current_config(&args.common_args.id).await?;
    config = override_config(config, OverrideConfig::from(args.clone()));
    log::debug!("Using config: {:#?}", config);

    if config.network_requester.open_proxy {
        println!(
            "\n\nYOU HAVE STARTED IN 'OPEN PROXY' MODE. ANYONE WITH YOUR CLIENT ADDRESS \
                CAN MAKE REQUESTS FROM YOUR MACHINE. PLEASE QUIT IF YOU DON'T UNDERSTAND WHAT \
                YOU'RE DOING.\n\n"
        );
    }

    if config.network_requester.enabled_statistics {
        println!(
            "\n\nTHE NETWORK REQUESTER STATISTICS ARE ENABLED. IT WILL COLLECT AND SEND \
                ANONYMIZED STATISTICS TO A CENTRAL SERVER. PLEASE QUIT IF YOU DON'T WANT \
                THIS TO HAPPEN AND START WITHOUT THE {ENABLE_STATISTICS} FLAG .\n\n"
        );
    }

    if !version_check(&config) {
        error!("failed the local version check");
        return Err(NetworkRequesterError::FailedLocalVersionCheck);
    }

    log::info!("Starting socks5 service provider");
    let mut server = crate::core::NRServiceProviderBuilder::new(config);
    if let Some(custom_mixnet) = &args.common_args.custom_mixnet {
        server = server.with_stored_topology(custom_mixnet)?
    }

    server.run_service_provider().await
}
