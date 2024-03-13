// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use clap::ValueEnum;
use nym_node::error::NymNodeError;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum NodeType {
    Mixnode,

    Gateway,
}

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Type of node (mixnode or gateway) to migrate into a nym-node.
    #[clap(long)]
    node_type: NodeType,

    /// Path to a configuration file of a node that's going to get migrated.
    #[clap(long)]
    config_file: PathBuf,
}

pub(crate) async fn execute(args: Args) -> Result<(), NymNodeError> {
    println!("args: {args:#?}");
    todo!()
}
