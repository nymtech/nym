// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::*;
use crate::config::Config;
use crate::node::MixNode;
use config::NymConfig;

#[derive(Args)]
pub(crate) struct NodeDetails {
    /// The id of the mixnode you want to show details for
    #[clap(long)]
    id: String,
}

pub(crate) fn execute(args: &NodeDetails) {
    let config = match Config::load_from_file(Some(&args.id)) {
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

    MixNode::new(config).print_node_details()
}
