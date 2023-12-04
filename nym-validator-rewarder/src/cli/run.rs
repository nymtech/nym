// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::try_load_current_config;
use crate::error::NymRewarderError;
use crate::rewarder::Rewarder;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Specifies custom location for the configuration file of nym validators rewarder.
    custom_config_path: Option<PathBuf>,
}

pub(crate) async fn execute(args: Args) -> Result<(), NymRewarderError> {
    let config = try_load_current_config(&args.custom_config_path)?.with_override(args);

    Rewarder::new(config).run().await
}
