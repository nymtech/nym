// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::ConfigOverridableArgs;
use crate::config::{Config, default_config_filepath};
use anyhow::Context;
use nym_config::defaults::NymNetworkDetails;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, nyxd};
use tracing::info;

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

    let network = NymNetworkDetails::new_from_env();
    let client_config = nyxd::Config::try_from_nym_network_details(&network)
        .context("failed to construct a validator client config from the network environment")?;
    let signing_client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
        client_config,
        config.nyxd_url.as_str(),
        config.mnemonic.clone(),
    )
    .context("failed to connect to nyxd")?;

    info!(
        "connected to nyxd at {} as {}",
        config.nyxd_url,
        signing_client.address()
    );
    Ok(())
}
