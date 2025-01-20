// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod ecash;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Internal {
    #[clap(subcommand)]
    pub command: InternalCommands,
}

#[derive(Debug, Subcommand)]
pub enum InternalCommands {
    /// Ecash related internal commands
    Ecash(ecash::InternalEcash),
}
