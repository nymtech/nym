// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod decode_node_key;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsNymNodeKeys {
    #[clap(subcommand)]
    pub command: MixnetOperatorsNymNodeKeysCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsNymNodeKeysCommands {
    /// Decode a Nym Node key
    DecodeNodeKey(decode_node_key::Args),
}
