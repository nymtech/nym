// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::ConfigOverridableArgs;
use crate::config::{Config, default_config_filepath};
use anyhow::{Context, bail};
use nym_config::defaults::NymNetworkDetails;
use tracing::info;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[command(flatten)]
    config_override: ConfigOverridableArgs,

    /// Mnemonic of the mixnet contract's rewarding validator account.
    #[clap(long, env = "NYM_EPOCH_DRIVER_MNEMONIC", hide_env_values = true)]
    mnemonic: bip39::Mnemonic,

    /// Bearer token every request to the metrics route must carry.
    #[clap(
        long,
        env = "NYM_EPOCH_DRIVER_METRICS_BEARER_TOKEN",
        hide_env_values = true
    )]
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

    let network_nyxd_url = NymNetworkDetails::new_from_env()
        .endpoints
        .first()
        .context("the network environment names no nyxd endpoint")?
        .nyxd_url();

    let config = Config::new(network_nyxd_url, args.mnemonic, args.metrics_bearer_token)
        .with_overrides(args.config_override);
    config.validate()?;

    config.save_to_path(&path).with_context(|| {
        format!(
            "failed to save the configuration file to '{}'",
            path.display()
        )
    })?;

    info!("saved the configuration file to '{}'", path.display());
    Ok(())
}
