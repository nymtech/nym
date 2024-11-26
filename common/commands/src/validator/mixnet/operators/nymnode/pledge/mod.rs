// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod decrease_pledge;
pub mod increase_pledge;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsNymNodePledge {
    #[clap(subcommand)]
    pub command: MixnetOperatorsNymNodePledgeCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsNymNodePledgeCommands {
    /// Increase current pledge
    Increase(increase_pledge::Args),
    /// decrease current pledge
    Decrease(decrease_pledge::Args),
}
