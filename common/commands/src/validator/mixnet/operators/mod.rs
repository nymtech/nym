// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod gateway;
pub mod identity_key;
pub mod mixnode;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperators {
    #[clap(subcommand)]
    pub command: MixnetOperatorsCommands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsCommands {
    /// Manage your mixnode
    Mixnode(mixnode::MixnetOperatorsMixnode),
    /// Manage your gateway
    Gateway(gateway::MixnetOperatorsGateway),
    /// Sign messages using your private identity key
    IdentityKey(identity_key::MixnetOperatorsIdentityKey),
}
