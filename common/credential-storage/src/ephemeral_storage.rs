// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::backends::memory::CoconutCredentialManager;
use crate::error::StorageError;
use crate::models::{CoconutCredential, StoredIssuedCredential};
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

#[async_trait]
impl Storage for EphemeralStorage {
    type StorageError = StorageError;

    async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
        epoch_id: String,
    ) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .insert_coconut_credential(
                voucher_value,
                voucher_info,
                serial_number,
                binding_number,
                signature,
                epoch_id,
            )
            .await;

        Ok(())
    }

    async fn get_next_coconut_credential(&self) -> Result<CoconutCredential, StorageError> {
        let credential = self
            .coconut_credential_manager
            .get_next_coconut_credential()
            .await
            .ok_or(StorageError::NoCredential)?;

        Ok(credential)
    }

    async fn get_next_unspent_credential(
        &self,
    ) -> Result<StoredIssuedCredential, Self::StorageError> {
        todo!()
    }

    async fn consume_coconut_credential(&self, id: i64) -> Result<(), StorageError> {
        self.coconut_credential_manager
            .consume_coconut_credential(id)
            .await;

        Ok(())
    }
}
