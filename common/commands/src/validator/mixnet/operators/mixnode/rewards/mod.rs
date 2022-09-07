// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod claim_operator_reward;
pub mod vesting_claim_operator_reward;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsMixnodeRewards {
    #[clap(subcommand)]
    pub command: MixnetOperatorsMixnodeRewardsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsMixnodeRewardsCommands {
    /// Claim rewards
    Claim(claim_operator_reward::Args),
    /// Claim rewards for a mixnode bonded with locked tokens
    VestingClaim(vesting_claim_operator_reward::Args),
}
