// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
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

pub(crate) fn execute(args: &NodeDetails) {
    let config = match Config::read_from_default_path(&args.id) {
        Ok(cfg) => cfg,
        Err(err) => {
            error!(
                "Failed to load config for {}. Are you sure you have run `init` before? (Error was: {})",
                args.id,
                err,
            );
            return;
        }
    };

    MixNode::new(config).print_node_details(args.output)
}
