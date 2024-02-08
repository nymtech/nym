// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{StorableIssuedCredential, StoredIssuedCredential};
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait Storage: Send + Sync {
    type StorageError: Error;

    async fn insert_issued_credential<'a>(
        &self,
        bandwidth_credential: StorableIssuedCredential<'a>,
    ) -> Result<(), Self::StorageError>;

    /// Tries to retrieve one of the stored, unused credentials.
    async fn get_next_unspent_credential(
        &self,
    ) -> Result<StoredIssuedCredential, Self::StorageError>;

    /// Marks as consumed in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Id of the credential to be consumed.
    async fn consume_coconut_credential(&self, id: i64) -> Result<(), Self::StorageError>;
}
