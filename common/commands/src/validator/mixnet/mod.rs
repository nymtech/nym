// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod delegators;
pub mod operators;
pub mod query;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Mixnet {
    #[clap(subcommand)]
    pub command: MixnetCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetCommands {
    /// Query the mixnet directory
    Query(query::MixnetQuery),
    /// Manage your delegations
    Delegators(delegators::MixnetDelegators),
    /// Manage a mixnode or gateway you operate
    Operators(operators::MixnetOperators),
}
