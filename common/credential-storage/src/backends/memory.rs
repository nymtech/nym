// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{CredentialUsage, StoredIssuedCredential};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoconutCredentialManager {
    inner: Arc<RwLock<CoconutCredentialManagerInner>>,
}

#[derive(Default)]
struct CoconutCredentialManagerInner {
    credentials: Vec<StoredIssuedCredential>,
    credential_usage: Vec<CredentialUsage>,
    _next_id: i64,
}

impl CoconutCredentialManagerInner {
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

    pub async fn take_credentials(self) -> Vec<StoredIssuedCredential> {
        let mut inner = self.inner.write().await;
        inner.credential_usage = Vec::new();
        std::mem::take(&mut inner.credentials)
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
        guard.credential_usage.push(CredentialUsage {
            credential_id: id,
            gateway_id_bs58: gateway_id.to_string(),
        });
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
