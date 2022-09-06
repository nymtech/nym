// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod errors;
pub mod helpers;
pub mod sign;
pub mod verify;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Signature {
    #[clap(subcommand)]
    pub command: Option<SignatureCommands>,
}

#[derive(Debug, Subcommand)]
pub enum SignatureCommands {
    /// Sign a message
    Sign(crate::validator::signature::sign::Args),
    /// Verify a message
    Verify(crate::validator::signature::verify::Args),
}
