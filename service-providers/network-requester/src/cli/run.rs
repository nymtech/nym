// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::{try_load_current_config, version_check};
use crate::{
    cli::{override_config, OverrideConfig},
    error::NetworkRequesterError,
};
use clap::Args;
use log::error;
use nym_sphinx::addressing::clients::Recipient;

const ENABLE_STATISTICS: &str = "enable-statistics";

#[allow(clippy::struct_excessive_bools)]
#[derive(Args, Clone)]
pub(crate) struct Run {
    /// Id of the nym-mixnet-client we want to run.
    #[arg(long)]
    id: String,

    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[arg(long)]
    open_proxy: bool,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[arg(long)]
    enable_statistics: bool,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym
    /// aggregator client
    #[arg(long)]
    statistics_recipient: Option<String>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[arg(long)]
    enabled_credentials_mode: Option<bool>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[arg(long, hide = true)]
    fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[arg(long, hide = true)]
    no_cover: bool,

    /// Enable medium mixnet traffic, for experiments only.
    /// This includes things like disabling cover traffic, no per hop delays, etc.
    #[arg(long, hide = true)]
    medium_toggle: bool,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            description: None,
            nym_apis: None,
            fastmode: run_config.fastmode,
            no_cover: run_config.no_cover,
            medium_toggle: run_config.medium_toggle,
            nyxd_urls: None,
            enabled_credentials_mode: run_config.enabled_credentials_mode,
        }
    }
}

pub(crate) async fn execute(args: &Run) -> Result<(), NetworkRequesterError> {
    if args.open_proxy {
        println!(
            "\n\nYOU HAVE STARTED IN 'OPEN PROXY' MODE. ANYONE WITH YOUR CLIENT ADDRESS \
                CAN MAKE REQUESTS FROM YOUR MACHINE. PLEASE QUIT IF YOU DON'T UNDERSTAND WHAT \
                YOU'RE DOING.\n\n"
        );
    }

    if args.enable_statistics {
        println!(
            "\n\nTHE NETWORK REQUESTER STATISTICS ARE ENABLED. IT WILL COLLECT AND SEND \
                ANONYMIZED STATISTICS TO A CENTRAL SERVER. PLEASE QUIT IF YOU DON'T WANT \
                THIS TO HAPPEN AND START WITHOUT THE {ENABLE_STATISTICS} FLAG .\n\n"
        );
    }

    let mut config = try_load_current_config(&args.id)?;
    dbg!(&config);
    config = override_config(config, OverrideConfig::from(args.clone()));
    log::debug!("Using config: {:#?}", config);

    if !version_check(&config) {
        error!("failed the local version check");
        return Err(NetworkRequesterError::FailedLocalVersionCheck);
    }

    // TODO: consider incorporating statistics_recipient, open_proxy and enable_statistics in
    // `Config`.

    let stats_provider_addr = args
        .statistics_recipient
        .as_ref()
        .map(Recipient::try_from_base58_string)
        .transpose()
        .unwrap_or(None);

    log::info!("Starting socks5 service provider");
    let server = crate::core::NRServiceProviderBuilder::new(
        config,
        args.open_proxy,
        args.enable_statistics,
        stats_provider_addr,
    )
    .await;
    server.run_service_provider().await
}
