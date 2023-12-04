// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use std::path::PathBuf;

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

#[derive(Debug, clap::Args)]
pub struct ConfigOverridableArgs {
    //
}

pub(crate) fn execute(args: Args) -> Result<(), NymRewarderError> {
    let path = args
        .custom_config_path
        .clone()
        .unwrap_or(Config::default_location());

    if path.exists() && !args.force {
        return Err(NymRewarderError::ExistingConfig { path });
    }

    Config::new(args.mnemonic)
        .with_override(args.config_override)
        .save_to_path(&path)
        .map_err(|source| NymRewarderError::ConfigSaveFailure { path, source })?;

    Ok(())
}
