// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::helpers::ConfigArgs;
use crate::node::NymNode;
use nym_bin_common::output_format::OutputFormat;
use nym_node::config::upgrade_helpers::try_load_current_config;
use nym_node::error::NymNodeError;

#[derive(Debug, clap::Args)]
pub(crate) struct Args {
    #[clap(flatten)]
    pub(crate) config: ConfigArgs,

    /// Specify the output format of the node details (`text` or `json`)
    #[clap(
        short,
        long,
        default_value_t = OutputFormat::default(),
    )]
    pub(crate) output: OutputFormat,
}

pub async fn execute(args: Args) -> Result<(), NymNodeError> {
    let config = try_load_current_config(args.config.config_path()).await?;
    let details = NymNode::new(config).await?.display_details();
    args.output.to_stdout(&details);
    Ok(())
}
