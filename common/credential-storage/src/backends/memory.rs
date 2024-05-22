// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::CoinIndicesSignature;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoconutCredentialManager {
    inner: Arc<RwLock<EcashCredentialManagerInner>>,
}

#[derive(Default)]
struct EcashCredentialManagerInner {
    credentials: Vec<StoredIssuedCredential>,
    credential_usage: Vec<CredentialUsage>,
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
        credential_type: String,
        serialization_revision: u8,
        credential_data: &[u8],
        epoch_id: u32,
    ) {
        let mut inner = self.inner.write().await;
        let id = inner.next_id();
        inner.credentials.push(StoredIssuedCredential {
            id,
            serialization_revision,
            credential_data: credential_data.to_vec(),
            credential_type,
            epoch_id,
            expired: false,
        })
    }

    async fn bandwidth_voucher_spent(&self, id: i64) -> bool {
        self.inner
            .read()
            .await
            .credential_usage
            .iter()
            .any(|c| c.credential_id == id)
    }

    async fn freepass_spent(&self, id: i64, gateway_id: &str) -> bool {
        self.inner
            .read()
            .await
            .credential_usage
            .iter()
            .any(|c| c.credential_id == id && c.gateway_id_bs58 == gateway_id)
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub async fn get_next_unspect_bandwidth_voucher(&self) -> Option<StoredIssuedCredential> {
        let guard = self.inner.read().await;
        for credential in guard
            .credentials
            .iter()
            .filter(|c| c.credential_type == "BandwidthVoucher")
        {
            if !self.bandwidth_voucher_spent(credential.id).await {
                return Some(credential.clone());
            }
        }
        None
    }

    pub async fn get_next_unspect_freepass(
        &self,
        gateway_id: &str,
    ) -> Option<StoredIssuedCredential> {
        let guard = self.inner.read().await;
        for credential in guard
            .credentials
            .iter()
            .filter(|c| c.credential_type == "FreeBandwidthPass")
        {
            if credential.expired {
                continue;
            }
            if !self.freepass_spent(credential.id, gateway_id).await {
                return Some(credential.clone());
            }
        }
        None
    }

    /// Consumes in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Database id.
    pub async fn consume_coconut_credential(&self, id: i64, gateway_id: &str) {
        let mut guard = self.inner.write().await;
    /// Inserts provided coin_indices_signatures into the database.
    ///
    /// # Arguments
    ///
    /// * `epoch_id`: Id of the epoch.
    /// * `coin_indices_signatures` : The coin indices signatures for the epoch
    pub async fn insert_coin_indices_sig(&self, epoch_id: String, coin_indices_sig: String) {
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
    pub async fn is_coin_indices_sig_present(&self, epoch_id: String) -> bool {
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
    pub async fn get_coin_indices_sig(&self, epoch_id: String) -> Option<CoinIndicesSignature> {
        let guard = self.inner.read().await;
        guard
            .coin_indices_sig
            .iter()
            .find(|s| s.epoch_id == epoch_id)
            .cloned()
    }

    /// Marks the specified credential as expired
    ///
    /// # Arguments
    ///
    /// * `id`: Id of the credential to mark as expired.
    pub async fn mark_expired(&self, id: i64) {
        let mut creds = self.inner.write().await;
        if let Some(cred) = creds.credentials.get_mut(id as usize) {
            cred.expired = true;
        }
    }
}
