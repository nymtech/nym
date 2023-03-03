// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod create_family;
pub mod create_family_creation_sign_payload;

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
