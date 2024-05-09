// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::try_load_current_config;
use crate::{
    client::{config::Config, SocketClient},
    commands::{override_config, OverrideConfig},
    error::ClientError,
};
use clap::Args;
use log::*;
use nym_bin_common::version_checker::is_minor_version_compatible;
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
        }
    }
}

// this only checks compatibility between config the binary. It does not take into consideration
// network version. It might do so in the future.
fn version_check(cfg: &Config) -> bool {
    let binary_version = env!("CARGO_PKG_VERSION");
    let config_version = &cfg.base.client.version;
    if binary_version == config_version {
        true
    } else {
        warn!("The native-client binary has different version than what is specified in config file! {} and {}", binary_version, config_version);
        if is_minor_version_compatible(binary_version, config_version) {
            info!("but they are still semver compatible. However, consider running the `upgrade` command");
            true
        } else {
            error!("and they are semver incompatible! - please run the `upgrade` command before attempting `run` again");
            false
        }
    }
}

pub(crate) async fn execute(args: Run) -> Result<(), Box<dyn Error + Send + Sync>> {
    eprintln!("Starting client {}...", args.common_args.id);

    let mut config = try_load_current_config(&args.common_args.id).await?;
    config = override_config(config, OverrideConfig::from(args.clone()));

    if !version_check(&config) {
        error!("failed the local version check");
        return Err(Box::new(ClientError::FailedLocalVersionCheck));
    }

    SocketClient::new(config, args.common_args.custom_mixnet)
        .run_socket_forever(None)
        .await
}
