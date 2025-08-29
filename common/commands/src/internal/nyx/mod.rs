// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod force_advance_epoch;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct InternalNyx {
    #[clap(subcommand)]
    pub command: InternalNyxCommands,
}

#[derive(Debug, Subcommand)]
pub enum InternalNyxCommands {
    /// Attempt to force advance the current epoch
    ForceAdvanceEpoch(force_advance_epoch::Args),
}
