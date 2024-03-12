// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backends::memory::CoconutCredentialManager;
use crate::error::StorageError;
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

impl std::fmt::Debug for EphemeralStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EphemeralStorage {{ .. }}")
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
        Ok(self
            .coconut_credential_manager
            .get_next_unspent_credential()
            .await)
    }

    async fn consume_coconut_credential(&self, id: i64) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .consume_coconut_credential(id)
            .await;

        Ok(())
    }

    async fn mark_expired(&self, id: i64) -> Result<(), Self::StorageError> {
        self.coconut_credential_manager.mark_expired(id).await;

        Ok(())
    }
}
