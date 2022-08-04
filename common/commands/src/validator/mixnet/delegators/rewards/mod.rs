// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod claim_delegator_reward;
pub mod vesting_claim_delegator_reward;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetDelegatorsReward {
    #[clap(subcommand)]
    pub command: MixnetDelegatorsRewardCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetDelegatorsRewardCommands {
    /// Claim rewards accumulated during the delegation of unlocked tokens
    Claim(claim_delegator_reward::Args),
    /// Claim rewards accumulated during the delegation of locked tokens
    VestingClaim(vesting_claim_delegator_reward::Args),
}
