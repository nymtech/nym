// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::OverrideConfig;
use crate::support::config::build_config;
use crate::OutputFormat;
use clap::Args;
use std::error::Error;

#[derive(Args, Clone)]
pub struct NodeDetails {
    /// The id of the gateway you want to show details for
    #[clap(long)]
    id: String,
}

pub async fn execute(
    args: NodeDetails,
    output: OutputFormat,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = build_config(args.id.clone(), OverrideConfig::default())?;

    Ok(crate::node::create_gateway(config)
        .await
        .print_node_details(output)?)
}
