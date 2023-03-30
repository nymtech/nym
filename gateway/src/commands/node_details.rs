// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::OverrideConfig;
use crate::support::config::build_config;
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use std::error::Error;

#[derive(Args, Clone)]
pub struct NodeDetails {
    /// The id of the gateway you want to show details for
    #[clap(long)]
    id: String,

    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

pub async fn execute(args: NodeDetails) -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = build_config(args.id.clone(), OverrideConfig::default())?;

    Ok(crate::node::create_gateway(config)
        .await
        .print_node_details(args.output))
}
