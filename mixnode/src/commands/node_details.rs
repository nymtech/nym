// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::try_load_current_config;
use crate::node::MixNode;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;

#[derive(Args)]
pub(crate) struct NodeDetails {
    /// The id of the mixnode you want to show details for
    #[clap(long)]
    id: String,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub(crate) fn execute(args: &NodeDetails) -> anyhow::Result<()> {
    let config = try_load_current_config(&args.id)?;

    MixNode::new(config)?.print_node_details(args.output);
    Ok(())
}
