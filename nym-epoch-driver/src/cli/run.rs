// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::ConfigOverridableArgs;
use crate::config::{Config, default_config_filepath};
use crate::epoch_operations::EpochAdvancer;
use crate::http::HttpServer;
use anyhow::Context;
use nym_config::defaults::NymNetworkDetails;
use nym_task::{ShutdownManager, ShutdownToken};
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, nyxd};
use tracing::{error, info};

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[command(flatten)]
    config_override: ConfigOverridableArgs,

    /// Bearer token every request to the metrics route must carry.
    /// default: the config file's value
    #[clap(
        long,
        env = "NYM_EPOCH_DRIVER_METRICS_BEARER_TOKEN",
        hide_env_values = true
    )]
    metrics_bearer_token: Option<String>,
}

fn build_signing_client(
    nyxd_url: &str,
    mnemonic: bip39::Mnemonic,
) -> anyhow::Result<DirectSigningHttpRpcNyxdClient> {
    let network = NymNetworkDetails::new_from_env();
    let client_config = nyxd::Config::try_from_nym_network_details(&network)
        .context("failed to construct a validator client config from the network environment")?;

    let signing_client =
        DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(client_config, nyxd_url, mnemonic)
            .context("failed to connect to nyxd")?;

    info!(
        "connected to nyxd at {nyxd_url} as {}",
        signing_client.address()
    );

    Ok(signing_client)
}

async fn build_epoch_advancer(
    config: Config,
    shutdown_token: ShutdownToken,
) -> anyhow::Result<EpochAdvancer> {
    let signing_client = build_signing_client(config.nyxd_url.as_str(), config.mnemonic)?;

    EpochAdvancer::new(signing_client, config.debug.into(), shutdown_token).await
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let path = default_config_filepath();
    let mut config = Config::read_from_path(&path)
        .with_context(|| {
            format!(
                "failed to load the configuration file from '{}'. Has `init` been run?",
                path.display()
            )
        })?
        .with_overrides(args.config_override);
    if let Some(metrics_bearer_token) = args.metrics_bearer_token {
        config.metrics.bearer_token = metrics_bearer_token;
    }
    config.validate()?;

    let mut shutdown_manager = ShutdownManager::build_new_default()?;

    let http_bearer_token = config.metrics.take_bearer_token()?;
    let http_bind_address = config.http.bind_address;
    let epoch_advancer =
        build_epoch_advancer(config, shutdown_manager.clone_shutdown_token()).await?;
    let http_server = HttpServer::bind(
        http_bind_address,
        http_bearer_token,
        shutdown_manager.clone_shutdown_token(),
    )
    .await?;

    // start the tasks
    shutdown_manager.spawn(async move {
        if let Err(err) = http_server.run().await {
            error!("the http api failed: {err}");
        }
    });
    shutdown_manager.spawn(epoch_advancer.run());

    shutdown_manager.run_until_shutdown().await;
    Ok(())
}
