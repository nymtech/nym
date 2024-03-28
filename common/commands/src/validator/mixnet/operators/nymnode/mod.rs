// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod bond_nymnode;
pub mod keys;
pub mod nymnode_bonding_sign_payload;
pub mod pledge;
pub mod rewards;
pub mod settings;
pub mod unbond_nymnode;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsNymNode {
    #[clap(subcommand)]
    pub command: MixnetOperatorsNymNodeCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsNymNodeCommands {
    /// Operations for Nym Node keys
    Keys(keys::MixnetOperatorsNymNodeKeys),

    /// Manage your Nym Node operator rewards
    Rewards(rewards::MixnetOperatorsNymNodeRewards),

    /// Manage your Nym Node settings stored in the directory
    Settings(settings::MixnetOperatorsNymNodeSettings),

    /// Manage your Nym Node pledge
    Pledge(pledge::MixnetOperatorsNymNodePledge),

    /// Bond to a Nym Node
    Bond(bond_nymnode::Args),

    /// Unbond from a Nym Node
    Unbond(unbond_nymnode::Args),

    /// Create base58-encoded payload required for producing valid bonding signature.
    CreateNodeBondingSignPayload(nymnode_bonding_sign_payload::Args),
}
