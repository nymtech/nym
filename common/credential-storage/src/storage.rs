// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;

use crate::StorageError;

#[async_trait]
pub trait Storage: Send + Sync {
    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `signature`: Coconut credential in the form of a signature.
    async fn insert_coconut_credential(&self, credential: String) -> Result<(), StorageError>;

    /// Tries to retrieve one of the stored, unused credentials.
    async fn get_next_coconut_credential(&self) -> Result<Option<String>, StorageError>;

    /// Removes from the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `signature`: Coconut credential in the form of a signature.
    async fn remove_coconut_credential(&self, credential: String) -> Result<(), StorageError>;
}
