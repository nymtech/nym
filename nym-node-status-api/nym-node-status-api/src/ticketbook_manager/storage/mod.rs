// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::Storage;
use nym_credential_proxy_lib::error::CredentialProxyError;
use nym_credential_proxy_lib::storage::traits::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    GlobalEcashDataCache, VersionedSerialise,
};
use nym_validator_client::nym_api::EpochId;
use time::Date;

#[derive(Clone)]
pub(crate) struct TicketbookManagerStorage {
    storage: Storage,
}

impl GlobalEcashDataCache for TicketbookManagerStorage {
    async fn get_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<EpochVerificationKey>, CredentialProxyError> {
        let Some(raw) = self
            .storage
            .get_master_verification_key(epoch_id as i32)
            .await?
        else {
            return Ok(None);
        };

        let deserialised =
            EpochVerificationKey::try_unpack(&raw.serialised_key, raw.serialization_revision)
                .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    async fn insert_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), CredentialProxyError> {
        let packed = key.pack();
        Ok(self
            .storage
            .insert_master_verification_key(
                packed.revision as i16,
                key.epoch_id as i32,
                &packed.data,
            )
            .await?)
    }

    async fn get_master_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedCoinIndicesSignatures>, CredentialProxyError> {
        let Some(raw) = self
            .storage
            .get_master_coin_index_signatures(epoch_id as i32)
            .await?
        else {
            return Ok(None);
        };

        let deserialised = AggregatedCoinIndicesSignatures::try_unpack(
            &raw.serialised_signatures,
            raw.serialization_revision,
        )
        .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    async fn insert_master_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), CredentialProxyError> {
        let packed = signatures.pack();
        self.storage
            .insert_master_coin_index_signatures(
                packed.revision as i16,
                signatures.epoch_id as i32,
                &packed.data,
            )
            .await?;
        Ok(())
    }

    async fn get_master_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<Option<AggregatedExpirationDateSignatures>, CredentialProxyError> {
        let Some(raw) = self
            .storage
            .get_master_expiration_date_signatures(expiration_date, epoch_id as i32)
            .await?
        else {
            return Ok(None);
        };

        let deserialised = AggregatedExpirationDateSignatures::try_unpack(
            &raw.serialised_signatures,
            raw.serialization_revision,
        )
        .map_err(|err| CredentialProxyError::database_inconsistency(err.to_string()))?;
        Ok(Some(deserialised))
    }

    async fn insert_master_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), CredentialProxyError> {
        let packed = signatures.pack();
        self.storage
            .insert_master_expiration_date_signatures(
                packed.revision as i16,
                signatures.epoch_id as i32,
                signatures.expiration_date,
                &packed.data,
            )
            .await?;
        Ok(())
    }
}
