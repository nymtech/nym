// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod issue_credentials;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Coconut {
    #[clap(subcommand)]
    pub command: CoconutCommands,
}

#[derive(Debug, Subcommand)]
pub enum CoconutCommands {
    IssueCredentials(issue_credentials::Args),
}
