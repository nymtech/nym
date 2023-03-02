// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};
use cosmrs::AccountId;
use cosmwasm_std::Addr;

pub mod create_family;
pub mod create_family_creation_sign_payload;

// TODO: perhaps it should be moved to some global common crate?
fn account_id_to_cw_addr(account_id: &AccountId) -> Addr {
    // the call to unchecked is fine here as we're converting directly from `AccountId`
    // which must have been a valid bech32 address
    Addr::unchecked(account_id.as_ref())
}

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct MixnetOperatorsMixnodeFamilies {
    #[clap(subcommand)]
    pub command: MixnetOperatorsMixnodeFamiliesCommands,
}

#[derive(Debug, Subcommand)]
pub enum MixnetOperatorsMixnodeFamiliesCommands {
    /// Create family
    CreateFamily(create_family::Args),
    /// Create message payload that is required to get signed to create a family
    CreateFamilyCreationSignPayload(create_family_creation_sign_payload::Args),
}
