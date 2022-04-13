// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;

use crate::{CoconutCredential, StorageError};

#[async_trait]
pub trait Storage: Send + Sync {
    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `signature`: Coconut credential in the form of a signature.
    async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
    ) -> Result<(), StorageError>;

    /// Tries to retrieve one of the stored, unused credentials.
    async fn get_next_coconut_credential(&self) -> Result<CoconutCredential, StorageError>;

    /// Removes from the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `signature`: Coconut credential in the form of a signature.
    async fn remove_coconut_credential(&self, id: i64) -> Result<(), StorageError>;
}
