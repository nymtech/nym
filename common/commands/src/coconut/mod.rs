// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod import_credential;
pub mod issue_credentials;
pub mod recover_credentials;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Coconut {
    #[clap(subcommand)]
    pub command: CoconutCommands,
}

#[derive(Debug, Subcommand)]
pub enum CoconutCommands {
    IssueCredentials(issue_credentials::Args),
    RecoverCredentials(recover_credentials::Args),
    ImportCredential(import_credential::Args),
}
