// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod block_time;
pub mod current_height;
pub mod get;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Block {
    #[clap(subcommand)]
    pub command: Option<BlockCommands>,
}

#[derive(Debug, Subcommand)]
pub enum BlockCommands {
    /// Gets a block's details and prints as JSON
    Get(crate::validator::block::get::Args),
    /// Gets the block time at a height
    Time(crate::validator::block::block_time::Args),
    /// Gets the current block height
    CurrentHeight(crate::validator::block::current_height::Args),
}
