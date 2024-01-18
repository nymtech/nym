// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod balance;
pub mod create;
pub mod pubkey;
pub mod send;
pub mod send_multiple;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Account {
    #[clap(subcommand)]
    pub command: Option<AccountCommands>,
}

#[derive(Debug, Subcommand)]
pub enum AccountCommands {
    /// Create a new mnemonic - note, this account does not appear on the chain until the account id is used in a transaction
    Create(crate::validator::account::create::Args),
    /// Gets the balance of an account
    Balance(crate::validator::account::balance::Args),
    /// Gets the public key of an account
    PubKey(crate::validator::account::pubkey::Args),
    /// Sends tokens to another account
    Send(crate::validator::account::send::Args),
    /// Batch multiple token sends
    SendMultiple(crate::validator::account::send_multiple::Args),
}
