// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{default_config_filepath, Config, ConfigBuilder};
use crate::error::NyxChainWatcherError;
use nym_config::save_unformatted_config_to_file;

#[derive(clap::Args, Debug)]
pub(crate) struct Args {}

pub(crate) async fn execute(_args: Args) -> Result<(), NyxChainWatcherError> {
    let config_path = default_config_filepath();
    let data_dir = Config::default_data_directory(&config_path)?;

    let builder = ConfigBuilder::new(config_path.clone(), data_dir);
    let config = builder.build();

    Ok(save_unformatted_config_to_file(&config, &config_path)?)
}
