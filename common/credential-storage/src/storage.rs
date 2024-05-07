// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CoinIndicesSignature;
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

    /// Tries to retrieve one of the stored, unused credentials,
    /// that is also not marked as expired
    async fn get_next_unspent_credential(
        &self,
        gateway_id: &str,
    ) -> Result<Option<StoredIssuedCredential>, Self::StorageError>;

    /// Marks as consumed in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Id of the credential to be consumed.
    /// * `gateway_id`: id of the gateway that received the credential.
    async fn consume_coconut_credential(
        &self,
        id: i64,
    /// Inserts provided coin_indices_signatures into the database.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    /// * `coin_indices_signatures` : The coin indices signatures for the epoch
    async fn insert_coin_indices_sig(
        &self,
        epoch_id: String,
        coin_indices_sig: String,
    ) -> Result<(), Self::StorageError>;

    /// Check if coin indices signatures are present for a given epoch
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    async fn is_coin_indices_sig_present(
        &self,
        epoch_id: String,
    ) -> Result<bool, Self::StorageError>;

    /// Get coin_indices_signatures of a given epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    async fn get_coin_indices_sig(
        &self,
        epoch_id: String,
    ) -> Result<CoinIndicesSignature, Self::StorageError>;
    /// Marks the specified credential as expired
    ///
    /// # Arguments
    ///
    /// * `id`: Id of the credential to mark as expired.
    async fn mark_expired(&self, id: i64) -> Result<(), Self::StorageError>;
}
