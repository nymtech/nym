// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::env::vars::NYMNODE_MODE_ARG;
use crate::node::bonding_information::BondingInformationV1;
use nym_bin_common::output_format::OutputFormat;
use nym_node::config::upgrade_helpers::try_load_current_config;
use nym_node::config::NodeMode;
use nym_node::error::NymNodeError;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    pub(crate) config: ConfigArgs,

    #[clap(
        long,
        value_enum,
        env = NYMNODE_MODE_ARG
    )]
    pub(crate) mode: Option<NodeMode>,

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
    let mut info = BondingInformationV1::try_load(&config)?;
    if let Some(mode) = args.mode {
        info = info.with_mode(mode)
    }
    args.output.to_stdout(&info);
    Ok(())
}
