// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::StoredIssuedCredential;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoconutCredentialManager {
    inner: Arc<RwLock<CoconutCredentialManagerInner>>,
}

#[derive(Default)]
struct CoconutCredentialManagerInner {
    data: Vec<StoredIssuedCredential>,
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

    pub async fn insert_issued_credential(
        &self,
        credential_type: String,
        serialization_revision: u8,
        credential_data: &[u8],
        epoch_id: u32,
    ) {
        let mut inner = self.inner.write().await;
        let id = inner.next_id();
        inner.data.push(StoredIssuedCredential {
            id,
            serialization_revision,
            credential_data: credential_data.to_vec(),
            credential_type,
            epoch_id,
            consumed: false,
        })
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub async fn get_next_unspent_credential(&self) -> Option<StoredIssuedCredential> {
        let creds = self.inner.read().await;
        creds.data.iter().find(|c| !c.consumed).cloned()
    }

    /// Consumes in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Database id.
    pub async fn consume_coconut_credential(&self, id: i64) {
        let mut creds = self.inner.write().await;
        if let Some(cred) = creds.data.get_mut(id as usize) {
            cred.consumed = true;
        }
    }
}
