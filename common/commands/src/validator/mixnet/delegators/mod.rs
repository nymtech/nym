// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod rewards;

pub mod delegate_to_mixnode;
pub mod query_for_delegations;
pub mod undelegate_from_mixnode;
pub mod vesting_delegate_to_mixnode;
pub mod vesting_undelegate_from_mixnode;
pub mod pledge_more;
pub mod vesting_pledge_more;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetDelegators {
    #[clap(subcommand)]
    pub command: MixnetDelegatorsCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetDelegatorsCommands {
    /// Lists current delegations
    List(query_for_delegations::Args),
    /// Manage rewards from delegations
    Rewards(rewards::MixnetDelegatorsReward),
    /// Delegate to a mixnode
    Delegate(delegate_to_mixnode::Args),
    /// Undelegate from a mixnode
    Undelegate(undelegate_from_mixnode::Args),
    /// Delegate to a mixnode with locked tokens
    DelegateVesting(vesting_delegate_to_mixnode::Args),
    /// Undelegate from a mixnode (when originally using locked tokens)
    UndelegateVesting(vesting_undelegate_from_mixnode::Args),
    /// Pledge more
    PledgeMore(pledge_more::Args),
    /// Pledge more with locked tokens
    PledgeMoreVesting(vesting_pledge_more::Args),
}
