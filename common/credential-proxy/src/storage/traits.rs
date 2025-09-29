// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::storage::CredentialProxyStorage;
use nym_validator_client::nym_api::EpochId;
use time::Date;

pub use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
pub use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};

// we use it in our code so it's fine
#[allow(async_fn_in_trait)]
pub trait GlobalEcashDataCache {
    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochVerificationKey>, CredentialProxyError>;

    async fn insert_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), CredentialProxyError>;

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedCoinIndicesSignatures>, CredentialProxyError>;

    async fn insert_master_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), CredentialProxyError>;

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedExpirationDateSignatures>, CredentialProxyError>;

    async fn insert_master_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), CredentialProxyError>;
}

impl GlobalEcashDataCache for CredentialProxyStorage {
    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochVerificationKey>, CredentialProxyError> {
        self.get_master_verification_key(epoch_id).await
    }

    async fn insert_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), CredentialProxyError> {
        self.insert_master_verification_key(key).await
    }

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedCoinIndicesSignatures>, CredentialProxyError> {
        self.get_master_coin_index_signatures(epoch_id).await
    }

    async fn insert_master_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), CredentialProxyError> {
        self.insert_master_coin_index_signatures(signatures).await
    }

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedExpirationDateSignatures>, CredentialProxyError> {
        self.get_master_expiration_date_signatures(expiration_date, epoch_id)
            .await
    }

    async fn insert_master_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), CredentialProxyError> {
        self.insert_master_expiration_date_signatures(signatures)
            .await
    }
}
