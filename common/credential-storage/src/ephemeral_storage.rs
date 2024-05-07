// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{self, Debug, Formatter};

use crate::backends::memory::CoconutCredentialManager;
use crate::error::StorageError;
use crate::models::CoinIndicesSignature;
use crate::models::{StorableIssuedCredential, StoredIssuedCredential};
use crate::storage::Storage;
use async_trait::async_trait;

pub type EphemeralCredentialStorage = EphemeralStorage;

// note that clone here is fine as upon cloning the same underlying pool will be used
#[derive(Clone)]
pub struct EphemeralStorage {
    coconut_credential_manager: CoconutCredentialManager,
}

impl Default for EphemeralStorage {
    fn default() -> Self {
        EphemeralStorage {
            coconut_credential_manager: CoconutCredentialManager::new(),
        }
    }
}

impl Debug for EphemeralStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "EphemeralStorage")
    }
}

#[async_trait]
impl Storage for EphemeralStorage {
    type StorageError = StorageError;

    async fn insert_issued_credential<'a>(
        &self,
        bandwidth_credential: StorableIssuedCredential<'a>,
    ) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .insert_issued_credential(
                bandwidth_credential.credential_type,
                bandwidth_credential.serialization_revision,
                bandwidth_credential.credential_data,
                bandwidth_credential.epoch_id,
            )
            .await;
        Ok(())
    }

    async fn get_next_unspent_credential(
        &self,
    ) -> Result<Option<StoredIssuedCredential>, Self::StorageError> {
        // first try to get a free pass if available, otherwise fallback to bandwidth voucher
        let maybe_freepass = self
            .coconut_credential_manager
            .get_next_unspent_freepass()
            .await;
        if maybe_freepass.is_some() {
            return Ok(maybe_freepass);
        }

        Ok(self
            .coconut_credential_manager
            .get_next_unspent_ticketbook()
            .await)
    }

    async fn update_issued_credential<'a>(
        &self,
        bandwidth_credential: StorableIssuedCredential<'a>,
        id: i64,
        consumed: bool,
    ) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .update_issued_credential(bandwidth_credential.credential_data, id, consumed)
            .await;
        Ok(())
    }

    async fn insert_coin_indices_sig(
        &self,
        epoch_id: i64,
        coin_indices_sig: String,
    ) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .insert_coin_indices_sig(epoch_id, coin_indices_sig)
            .await;
        Ok(())
    }

    async fn is_coin_indices_sig_present(&self, epoch_id: i64) -> Result<bool, StorageError> {
        Ok(self
            .coconut_credential_manager
            .is_coin_indices_sig_present(epoch_id)
            .await)
    }

    async fn get_coin_indices_sig(
        &self,
        epoch_id: i64,
    ) -> Result<CoinIndicesSignature, StorageError> {
        self.coconut_credential_manager
            .get_coin_indices_sig(epoch_id)
            .await
            .ok_or(StorageError::NoSignatures { epoch_id })
    }

    async fn mark_expired(&self, id: i64) -> Result<(), Self::StorageError> {
        self.coconut_credential_manager.mark_expired(id).await;

        Ok(())
    }
}
