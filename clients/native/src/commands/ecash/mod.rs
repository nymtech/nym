// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};
use std::error::Error;

pub(crate) mod import_coin_index_signatures;
pub(crate) mod import_credential;
pub(crate) mod import_expiration_date_signatures;
pub(crate) mod import_master_verification_key;
pub(crate) mod show_ticketbooks;

#[derive(Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Ecash {
    #[clap(subcommand)]
    pub command: EcashCommands,
}

impl Ecash {
    pub async fn execute(self) -> Result<(), Box<dyn Error + Send + Sync>> {
        match self.command {
            EcashCommands::ShowTicketBooks(args) => show_ticketbooks::execute(args).await?,
            EcashCommands::ImportTicketBook(args) => import_credential::execute(args).await?,
            EcashCommands::ImportCoinIndexSignatures(args) => {
                import_coin_index_signatures::execute(args).await?
            }
            EcashCommands::ImportExpirationDateSignatures(args) => {
                import_expiration_date_signatures::execute(args).await?
            }
            EcashCommands::ImportMasterVerificationKey(args) => {
                import_master_verification_key::execute(args).await?
            }
        }
        Ok(())
    }
}

#[derive(Subcommand)]
pub enum EcashCommands {
    /// Display information associated with the imported ticketbooks,
    ShowTicketBooks(show_ticketbooks::Args),

    /// Import a pre-generated ticketbook
    ImportTicketBook(import_credential::Args),

    /// Import coin index signatures needed for ticketbooks
    ImportCoinIndexSignatures(import_coin_index_signatures::Args),

    /// Import expiration date signatures needed for ticketbooks
    ImportExpirationDateSignatures(import_expiration_date_signatures::Args),

    /// Import master verification key needed for ticketbooks
    ImportMasterVerificationKey(import_master_verification_key::Args),
}
