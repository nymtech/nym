// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};
use nym_client_core::cli_helpers::client_import_coin_index_signatures::CommonClientImportCoinIndexSignaturesArgs;
use nym_client_core::cli_helpers::client_import_credential::CommonClientImportTicketBookArgs;
use nym_client_core::cli_helpers::client_import_expiration_date_signatures::CommonClientImportExpirationDateSignaturesArgs;
use nym_client_core::cli_helpers::client_import_master_verification_key::CommonClientImportMasterVerificationKeyArgs;
use nym_ip_packet_router::error::IpPacketRouterError;

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
    pub async fn execute(self) -> Result<(), IpPacketRouterError> {
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
    ImportTicketBook(CommonClientImportTicketBookArgs),

    /// Import coin index signatures needed for ticketbooks
    ImportCoinIndexSignatures(CommonClientImportCoinIndexSignaturesArgs),

    /// Import expiration date signatures needed for ticketbooks
    ImportExpirationDateSignatures(CommonClientImportExpirationDateSignaturesArgs),

    /// Import master verification key needed for ticketbooks
    ImportMasterVerificationKey(CommonClientImportMasterVerificationKeyArgs),
}
