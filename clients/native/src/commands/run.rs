// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::try_load_current_config;
use crate::{
    client::SocketClient,
    commands::{override_config, OverrideConfig},
};
use clap::Args;
use nym_client_core::cli_helpers::client_run::CommonClientRunArgs;
use std::error::Error;
use std::net::IpAddr;

#[derive(Args, Clone)]
pub(crate) struct Run {
    #[command(flatten)]
    common_args: CommonClientRunArgs,

    /// Whether to not start the websocket
    #[clap(long)]
    disable_socket: Option<bool>,

    /// Port for the socket to listen on
    #[clap(short, long)]
    port: Option<u16>,

    /// Ip for the socket (if applicable) to listen for requests.
    #[clap(long)]
    host: Option<IpAddr>,
}

impl From<Run> for OverrideConfig {
    fn from(run_config: Run) -> Self {
        OverrideConfig {
            nym_apis: run_config.common_args.nym_apis,
            disable_socket: run_config.disable_socket,
            port: run_config.port,
            host: run_config.host,
            fastmode: run_config.common_args.fastmode,
            no_cover: run_config.common_args.no_cover,
            nyxd_urls: run_config.common_args.nyxd_urls,
            enabled_credentials_mode: run_config.common_args.enabled_credentials_mode,
            stats_reporting_address: run_config.common_args.stats_reporting_address,
            forget_me: run_config.common_args.forget_me.into(),
        }
    }
}

pub(crate) async fn execute(args: Run) -> Result<(), Box<dyn Error + Send + Sync>> {
    eprintln!("Starting client {}...", args.common_args.id);

    let mut config = try_load_current_config(&args.common_args.id).await?;
    config = override_config(config, OverrideConfig::from(args.clone()));

    SocketClient::new(config, args.common_args.custom_mixnet)
        .run_socket_forever()
        .await
}
