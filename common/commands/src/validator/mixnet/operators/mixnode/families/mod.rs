// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod create_family;
pub mod create_family_join_permit_sign_payload;
pub mod join_family;
pub mod kick_family_member;
pub mod leave_family;

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

    /// Join family
    JoinFamily(join_family::Args),

    /// Leave family,
    LeaveFamily(leave_family::Args),

    /// Kick family member
    KickFamilyMember(kick_family_member::Args),

    /// Create a message payload that is required to get signed in order to obtain a permit for joining family
    CreateFamilyJoinPermitSignPayload(create_family_join_permit_sign_payload::Args),
}
