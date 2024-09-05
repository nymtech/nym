// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod import_credits;
pub mod issue_credits;
pub mod recover_credits;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Zknym {
    #[clap(subcommand)]
    pub command: ZknymCommands,
}

#[derive(Debug, Subcommand)]
pub enum ZknymCommands {
    IssueCredits(issue_credits::Args),
    RecoverCredits(recover_credits::Args),
    ImportCredits(import_credits::Args),
}
