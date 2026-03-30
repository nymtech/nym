// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{ArgGroup, Args};
use nym_credentials_interface::TicketType;
use nym_sdk::mixnet::{
    CredentialStorage, DisconnectedMixnetClient, Ephemeral, MixnetClientStorage, OnDiskPersistent,
};

use crate::{
    common::bandwidth_helpers::{acquire_bandwidth, import_bandwidth},
    types::AttachedTicketMaterials,
};

#[derive(Debug, Args)]
pub struct CredentialArgs {
    /// Serialized credential data
    #[arg(long)]
    pub ticket_materials: String,

    /// Version of the serialized credential
    #[arg(long, default_value_t = 1)]
    pub ticket_materials_revision: u8,
}

impl CredentialArgs {
    pub async fn import_credential(
        self,
        mixnet_client: &DisconnectedMixnetClient<Ephemeral>,
    ) -> anyhow::Result<()> {
        let tickets_materials = AttachedTicketMaterials::from_serialised_string(
            self.ticket_materials,
            self.ticket_materials_revision,
        )?;
        let bandwidth_import = mixnet_client.begin_bandwidth_import();
        import_bandwidth(bandwidth_import, tickets_materials).await?;
        Ok(())
    }
}

/// Two ways to inject credentials when not running as agent
/// 1. Mnemonic : expected to be used on lower envs
///     - mnemonic
/// 2. Mock ecash : expected to be used for local setups
///     - use_mock_ecash
#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("credential_mode")
        .args(["use_mock_ecash","mnemonic"])
        .required(true)
        .multiple(false)
))]
pub struct CredentialMode {
    /// Use mock ecash credentials for testing (requires gateway with --lp-use-mock-ecash)
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub use_mock_ecash: bool,

    /// Mnemonic to get credentials from the blockchain. It needs NYMs.
    #[arg(long)]
    pub mnemonic: Option<String>,
}

impl CredentialMode {
    pub async fn acquire(
        &self,
        disconnected_mixnet_client: &DisconnectedMixnetClient<OnDiskPersistent>,
        storage: &OnDiskPersistent,
    ) -> anyhow::Result<()> {
        // Return immediately as there is nothing to do
        if self.use_mock_ecash {
            return Ok(());
        }
        let ticketbook_count = storage
            .credential_store()
            .get_ticketbooks_info()
            .await?
            .len();
        tracing::info!("Credential store contains {} ticketbooks", ticketbook_count);

        if ticketbook_count < 1 {
            let mnemonic = self.mnemonic.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "We are not using mock ecash and mnemonic is not set, this should not happen"
                )
            })?;
            for ticketbook_type in [
                TicketType::V1MixnetEntry,
                TicketType::V1WireguardEntry,
                TicketType::V1WireguardExit,
            ] {
                acquire_bandwidth(mnemonic, disconnected_mixnet_client, ticketbook_type).await?;
            }
        }
        Ok(())
    }
}
