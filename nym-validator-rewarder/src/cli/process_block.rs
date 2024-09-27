// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::{try_load_current_config, ConfigOverridableArgs};
use crate::error::NymRewarderError;
use nyxd_scraper::NyxdScraper;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    config_override: ConfigOverridableArgs,

    /// Height of the block we want to process
    #[clap(long)]
    height: u32,

    /// Specifies custom location for the configuration file of nym validators rewarder.
    #[clap(long)]
    custom_config_path: Option<PathBuf>,
}

pub(crate) async fn execute(args: Args) -> Result<(), NymRewarderError> {
    let config =
        try_load_current_config(&args.custom_config_path)?.with_override(args.config_override);

    NyxdScraper::new(config.scraper_config())
        .await?
        .process_single_block(args.height)
        .await?;
    Ok(())
}
