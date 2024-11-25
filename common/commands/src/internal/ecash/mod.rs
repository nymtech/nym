// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod generate_keypair;
pub mod withdrawal_request;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct InternalEcash {
    #[clap(subcommand)]
    pub command: InternalEcashCommands,
}

#[derive(Debug, Subcommand)]
pub enum InternalEcashCommands {
    /// Generate a dummy withdrawal request
    GenerateWithdrawalRequest(withdrawal_request::Args),

    /// Generate dummy ecash keypair
    GenerateKeypair(generate_keypair::Args),
}
