// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod generate_ticket;
pub mod import_coin_index_signatures;
pub mod import_expiration_date_signatures;
pub mod import_master_verification_key;
pub mod import_ticket_book;
pub mod issue_ticket_book;
pub mod recover_ticket_book;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Ecash {
    #[clap(subcommand)]
    pub command: EcashCommands,
}

#[derive(Debug, Subcommand)]
pub enum EcashCommands {
    IssueTicketBook(issue_ticket_book::Args),
    RecoverTicketBook(recover_ticket_book::Args),
    ImportTicketBook(import_ticket_book::Args),
    GenerateTicket(generate_ticket::Args),
    ImportCoinIndexSignatures(import_coin_index_signatures::Args),
    ImportExpirationDateSignatures(import_expiration_date_signatures::Args),
    ImportMasterVerificationKey(import_master_verification_key::Args),
}
