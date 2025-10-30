// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_credential_storage::storage::Storage;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;

use crate::{error::BandwidthControllerError, BandwidthController, PreparedCredential};

pub const DEFAULT_TICKETS_TO_SPEND: u32 = 1;

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
        // TODO: placeholder, implement by threading the actual data through
        Ok(None)
    }
}
