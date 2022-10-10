// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod decode_mixnode_key;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsMixnodeKeys {
    #[clap(subcommand)]
    pub command: MixnetOperatorsMixnodeKeysCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsMixnodeKeysCommands {
    /// Decode a mixnode key
    DecodeMixnodeKey(decode_mixnode_key::Args),
}
