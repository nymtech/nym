// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::config::upgrade_helpers::try_load_current_config;
use crate::error::NymNodeError;
use crate::node::bonding_information::BondingInformation;
use nym_bin_common::output_format::OutputFormat;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    pub(crate) config: ConfigArgs,

    /// Specify the output format of the bonding information (`text` or `json`)
    #[clap(
        short,
        long,
        default_value_t = OutputFormat::default(),
    )]
    pub(crate) output: OutputFormat,
}

pub async fn execute(args: Args) -> Result<(), NymNodeError> {
    let config = try_load_current_config(args.config.config_path()).await?;
    let info = BondingInformation::try_load(&config)?;
    args.output.to_stdout(&info);
    Ok(())
}
