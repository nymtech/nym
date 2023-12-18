// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::ConfigOverridableArgs;
use crate::config::{default_config_directory, default_data_directory, Config};
use crate::error::NymRewarderError;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    config_override: ConfigOverridableArgs,

    /// Specifies custom location for the configuration file of nym validators rewarder.
    #[clap(long)]
    custom_config_path: Option<PathBuf>,

    /// Mnemonic used for rewarding  operations
    #[clap(long)]
    mnemonic: bip39::Mnemonic,

    /// Overwrite existing configuration file.
    #[clap(long, short)]
    force: bool,
}

fn init_paths() -> io::Result<()> {
    fs::create_dir_all(default_data_directory())?;
    fs::create_dir_all(default_config_directory())
}

pub(crate) fn execute(args: Args) -> Result<(), NymRewarderError> {
    let path = args
        .custom_config_path
        .clone()
        .unwrap_or(Config::default_location());

    if path.exists() && !args.force {
        return Err(NymRewarderError::ExistingConfig { path });
    }

    init_paths().map_err(|source| NymRewarderError::PathInitialisationFailure { source })?;

    let config = Config::new(args.mnemonic).with_override(args.config_override);
    config.ensure_is_valid()?;

    config
        .save_to_path(&path)
        .map_err(|source| NymRewarderError::ConfigSaveFailure { path, source })?;

    Ok(())
}
