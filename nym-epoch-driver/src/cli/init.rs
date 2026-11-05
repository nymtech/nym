// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{Config, default_config_filepath};
use anyhow::{Context, bail};
use nym_config::defaults::NymNetworkDetails;
use tracing::info;
use url::Url;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Endpoint to the nyxd instance the driver queries and sends its transactions through.
    /// default: the network environment's nyxd endpoint
    #[clap(long)]
    nyxd_url: Option<Url>,

    /// Mnemonic of the mixnet contract's rewarding validator account.
    #[clap(long)]
    mnemonic: bip39::Mnemonic,

    /// Bearer token every request to the metrics route must carry.
    #[clap(long)]
    metrics_bearer_token: String,
}

pub(crate) fn execute(args: Args) -> anyhow::Result<()> {
    let path = default_config_filepath();
    if path.exists() {
        bail!(
            "there already exists a configuration file at '{}'. If you intend to replace it, you need to remove it first",
            path.display()
        )
    }

    let nyxd_url = match args.nyxd_url {
        Some(nyxd_url) => nyxd_url,
        None => NymNetworkDetails::new_from_env()
            .endpoints
            .first()
            .context("the network environment names no nyxd endpoint")?
            .nyxd_url(),
    };

    Config::new(nyxd_url, args.mnemonic, args.metrics_bearer_token)
        .save_to_path(&path)
        .with_context(|| {
            format!(
                "failed to save the configuration file to '{}'",
                path.display()
            )
        })?;

    info!("saved the configuration file to '{}'", path.display());
    Ok(())
}
