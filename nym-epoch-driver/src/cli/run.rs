// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{Config, default_config_filepath};
use anyhow::Context;
use nym_config::defaults::NymNetworkDetails;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, nyxd};
use tracing::info;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Endpoint to the nyxd instance the driver queries and sends its transactions through.
    /// default: None - config value will be used instead
    #[clap(long)]
    nyxd_url: Option<Url>,
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let path = default_config_filepath();
    let mut config = Config::read_from_path(&path).with_context(|| {
        format!(
            "failed to load the configuration file from '{}'. Has `init` been run?",
            path.display()
        )
    })?;
    if let Some(nyxd_url) = args.nyxd_url {
        config.nyxd_url = nyxd_url;
    }

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
