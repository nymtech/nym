// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_credential_storage::storage::Storage;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;

use crate::{error::BandwidthControllerError, BandwidthController, PreparedCredential};

pub const DEFAULT_TICKETS_TO_SPEND: u32 = 1;

// TODO: this does not really belong here
pub const UPGRADE_MODE_JWT_TYPE: &str = "UPGRADE_MODE_JWT";

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait BandwidthTicketProvider: Send + Sync {
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
    ) -> Result<PreparedCredential, BandwidthControllerError>;

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C, St> BandwidthTicketProvider for BandwidthController<C, St>
where
    C: DkgQueryClient + Sync + Send,
    St: nym_credential_storage::storage::Storage,
    <St as Storage>::StorageError: Send + Sync + 'static,
{
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
    ) -> Result<PreparedCredential, BandwidthControllerError> {
        self.prepare_ecash_ticket(ticket_type, gateway_id.to_bytes(), tickets_to_spend)
            .await
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        let Some(emergency_credential) =
            self.get_emergency_credential(UPGRADE_MODE_JWT_TYPE).await?
        else {
            return Ok(None);
        };
        // upgrade mode credential is just a simple stringified JWT
        let token = String::from_utf8(emergency_credential.content).map_err(|err| {
            BandwidthControllerError::CredentialStorageError(Box::new(format!(
                "malformed upgrade mode token: {err}"
            )))
        })?;
        Ok(Some(token))
    }
}
