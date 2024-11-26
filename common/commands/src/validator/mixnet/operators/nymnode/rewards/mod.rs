// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod claim_operator_reward;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsNymNodeRewards {
    #[clap(subcommand)]
    pub command: MixnetOperatorsNymNodeRewardsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsNymNodeRewardsCommands {
    /// Claim rewards
    Claim(claim_operator_reward::Args),
}
