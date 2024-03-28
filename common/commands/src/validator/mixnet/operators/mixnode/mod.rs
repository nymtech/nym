// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod bond_mixnode;
pub mod decrease_pledge;
pub mod keys;
pub mod migrate_vested_mixnode;
pub mod mixnode_bonding_sign_payload;
pub mod nymnode_migration;
pub mod pledge_more;
pub mod rewards;
pub mod settings;
pub mod unbond_mixnode;
pub mod vesting_bond_mixnode;
pub mod vesting_decrease_pledge;
pub mod vesting_pledge_more;
pub mod vesting_unbond_mixnode;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsMixnode {
    #[clap(subcommand)]
    pub command: MixnetOperatorsMixnodeCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsMixnodeCommands {
    /// Operations for mixnode keys
    Keys(keys::MixnetOperatorsMixnodeKeys),
    /// Manage your mixnode operator rewards
    Rewards(rewards::MixnetOperatorsMixnodeRewards),
    /// Manage your mixnode settings stored in the directory
    Settings(settings::MixnetOperatorsMixnodeSettings),
    /// Bond to a mixnode
    Bond(bond_mixnode::Args),
    /// Unbond from a mixnode
    Unbond(unbond_mixnode::Args),
    /// Bond to a mixnode with locked tokens
    BondVesting(vesting_bond_mixnode::Args),
    /// Unbond from a mixnode (when originally using locked tokens)
    UnbondVesting(vesting_unbond_mixnode::Args),
    /// Create base58-encoded payload required for producing valid bonding signature.
    CreateMixnodeBondingSignPayload(mixnode_bonding_sign_payload::Args),
    /// Pledge more
    PledgeMore(pledge_more::Args),
    /// Pledge more with locked tokens
    PledgeMoreVesting(vesting_pledge_more::Args),
    /// Decrease pledge
    DecreasePledge(decrease_pledge::Args),
    /// Decrease pledge with locked tokens
    DecreasePledgeVesting(vesting_decrease_pledge::Args),
    /// Migrate the mixnode to use liquid tokens
    MigrateVestedNode(migrate_vested_mixnode::Args),
    /// Migrate the mixnode into a Nym Node
    MigrateToNymnode(nymnode_migration::Args),
}
