// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CoinIndicesSignature;
use crate::models::StoredIssuedCredential;
use nym_ecash_time::{ecash_today, OffsetDateTime};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoconutCredentialManager {
    inner: Arc<RwLock<EcashCredentialManagerInner>>,
}

#[derive(Default)]
struct EcashCredentialManagerInner {
    credentials: Vec<StoredIssuedCredential>,
    coin_indices_sig: Vec<CoinIndicesSignature>,
    _next_id: i64,
}

impl EcashCredentialManagerInner {
    fn next_id(&mut self) -> i64 {
        let next = self._next_id;
        self._next_id += 1;
        next
    }
}

impl CoconutCredentialManager {
    /// Creates new empty instance of the `CoconutCredentialManager`.
    pub fn new() -> Self {
        CoconutCredentialManager {
            inner: Default::default(),
        }
    }

    pub async fn insert_issued_credential(
        &self,
        serialization_revision: u8,
        expiration_date: OffsetDateTime,
        credential_data: &[u8],
        epoch_id: u32,
    ) {
        let mut inner = self.inner.write().await;
        let id = inner.next_id();
        inner.credentials.push(StoredIssuedCredential {
            id,
            serialization_revision,
            credential_data: credential_data.to_vec(),
            expiration_date,
            epoch_id,
            consumed: false,
        })
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub async fn get_next_unspent_ticketbook(&self) -> Option<StoredIssuedCredential> {
        let deadline = ecash_today();
        let guard = self.inner.read().await;
        for credential in guard.credentials.iter() {
            if !credential.consumed && credential.expiration_date <= deadline {
                return Some(credential.clone());
            }
        }
        None
    }

    pub async fn update_issued_credential(
        &self,
        serialisation_revision: u8,
        updated_data: &[u8],
        id: i64,
        consumed: bool,
    ) {
        let mut guard = self.inner.write().await;
        if let Some(cred) = guard.credentials.get_mut(id as usize) {
            cred.serialization_revision = serialisation_revision;
            cred.credential_data = updated_data.to_vec();
            cred.consumed = consumed;
        }
    }

    /// Inserts provided coin_indices_signatures into the database.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    /// * `coin_indices_signatures` : The coin indices signatures for the epoch
    pub async fn insert_coin_indices_sig(&self, epoch_id: i64, coin_indices_sig: String) {
        let mut guard = self.inner.write().await;
        guard.coin_indices_sig.push(CoinIndicesSignature {
            epoch_id,
            signatures: coin_indices_sig,
        });
    }

    /// Check if coin indices signatures are present for a given epoch
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    pub async fn is_coin_indices_sig_present(&self, epoch_id: i64) -> bool {
        let guard = self.inner.read().await;
        guard
            .coin_indices_sig
            .iter()
            .any(|s| s.epoch_id == epoch_id)
    }

    /// Get coin_indices_signatures of a given epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    pub async fn get_coin_indices_sig(&self, epoch_id: i64) -> Option<CoinIndicesSignature> {
        let guard = self.inner.read().await;
        guard
            .coin_indices_sig
            .iter()
            .find(|s| s.epoch_id == epoch_id)
            .cloned()
    }
}
