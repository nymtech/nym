// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::helpers::try_load_current_config;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_gateway::helpers::node_details;

#[derive(Args, Clone)]
pub struct NodeDetails {
    /// The id of the gateway you want to show details for
    #[clap(long)]
    id: String,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn execute(args: NodeDetails) -> anyhow::Result<()> {
    let config = try_load_current_config(&args.id)?;
    args.output.to_stdout(&node_details(&config).await?);

    Ok(())
}
