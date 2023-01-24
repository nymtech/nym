// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;

use crate::models::CoconutCredential;
use crate::StorageError;

#[async_trait]
pub trait Storage: Send + Sync {
    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `voucher_value`: How much bandwidth is in the credential.
    /// * `voucher_info`: What type of credential it is.
    /// * `serial_number`: Serial number of the credential.
    /// * `binding_number`: Binding number of the credential.
    /// * `signature`: Coconut credential in the form of a signature.
    /// * `epoch_id`: The epoch when it was signed.
    async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
        epoch_id: String,
    ) -> Result<(), StorageError>;

    /// Tries to retrieve one of the stored, unused credentials.
    async fn get_next_coconut_credential(&self) -> Result<CoconutCredential, StorageError>;

    /// Marks as consumed in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Id of the credential to be consumed.
    async fn consume_coconut_credential(&self, id: i64) -> Result<(), StorageError>;
}
