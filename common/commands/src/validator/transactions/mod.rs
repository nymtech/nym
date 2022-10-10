// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod get_transaction;
pub mod query_transactions;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Transactions {
    #[clap(subcommand)]
    pub command: Option<TransactionsCommands>,
}

#[derive(Debug, Subcommand)]
pub enum TransactionsCommands {
    /// Get a transaction by hash or block height
    Get(crate::validator::transactions::get_transaction::Args),
    /// Query for transactions
    Query(crate::validator::transactions::query_transactions::Args),
}
