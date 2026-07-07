// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;

use nym_credentials::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
use nym_credentials::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::{Date, OffsetDateTime};
use nym_validator_client::nym_api::EpochId;

use crate::{error::BandwidthControllerError, PreparedCredential, PreparedCredentialMetadata};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait BandwidthTicketProvider: Send + Sync {
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError>;

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError>;

    async fn attempt_revert_spending(
        &self,
        metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError>;

    async fn close(&self);
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<T: BandwidthTicketProvider + ?Sized + Send> BandwidthTicketProvider for Box<T> {
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        (**self)
            .get_ecash_ticket(ticket_type, gateway_id, tickets_to_spend, spend_time)
            .await
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        (**self).get_upgrade_mode_token().await
    }

    async fn attempt_revert_spending(
        &self,
        metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        (**self).attempt_revert_spending(metadata).await
    }

    // For compatibility for now. Remove once BC is properly implemented in client repo
    async fn close(&self) {
        (**self).close().await;
    }
}

// This isn't an associated type because
// a) it would make it dyn-incompatible and we want it
// b) BandwidthController will pack everything its own variant anyway

/// Error any fetcher implementation may return; the controller wraps it with context.
pub type FetcherError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait CredentialPublicDataFetcher: Send + Sync {
    async fn fetch_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<EpochVerificationKey, FetcherError>;

    async fn fetch_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<AggregatedCoinIndicesSignatures, FetcherError>;

    async fn fetch_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<AggregatedExpirationDateSignatures, FetcherError>;
}
