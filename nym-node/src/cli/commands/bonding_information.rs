// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::helpers::ConfigArgs;
use crate::node::bonding_information::BondingInformationV1;
use nym_bin_common::output_format::OutputFormat;
use nym_node::config::upgrade_helpers::try_load_current_config;
use nym_node::error::NymNodeError;

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
    let info = BondingInformationV1::try_load(&config)?;
    args.output.to_stdout(&info);
    Ok(())
}
