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

    /// Optional starting height for processing the blocks.
    /// If none is provided, the default behaviour will be applied.
    #[clap(long)]
    start_height: Option<u32>,

    /// Height of until we want to be processing the blocks
    #[clap(long)]
    stop_height: u32,

    /// Specifies custom location for the configuration file of nym validators rewarder.
    #[clap(long)]
    custom_config_path: Option<PathBuf>,
}

pub(crate) async fn execute(args: Args) -> Result<(), NymRewarderError> {
    if let Some(start_height) = args.start_height {
        if start_height > args.stop_height {
            eprintln!("the start height can't be larger than the stop height!");
            return Ok(());
        }
    }

    let config =
        try_load_current_config(&args.custom_config_path)?.with_override(args.config_override);

    NyxdScraper::new(config.scraper_config())
        .await?
        .process_block_range(args.start_height, args.stop_height)
        .await?;
    Ok(())
}
