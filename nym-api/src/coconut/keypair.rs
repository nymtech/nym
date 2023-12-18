// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_compact_ecash::scheme::keygen::{KeyPairAuth, SecretKeyAuth, VerificationKeyAuth};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Clone, Debug)]
pub struct KeyPair {
    inner: Arc<RwLock<Option<nym_coconut_interface::KeyPair>>>,
}

impl KeyPair {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get(&self) -> RwLockReadGuard<'_, Option<nym_coconut::KeyPair>> {
        self.inner.read().await
    }

    //same key, but in a different type
    pub async fn get_ecash(&self) -> Option<KeyPairAuth> {
        let coconut_key = self.inner.read().await;
        coconut_key.as_ref().map(|key| {
            let secret_key_auth = SecretKeyAuth::from_bytes(&key.secret_key().to_bytes()).unwrap();
            let verification_key_auth =
                VerificationKeyAuth::from_bytes(&key.verification_key().to_bytes()).unwrap();
            let index = key.index;

            KeyPairAuth::new(secret_key_auth, verification_key_auth, index)
        })
    }

    pub async fn set(&self, keypair: Option<nym_coconut_interface::KeyPair>) {
        let mut w_lock = self.inner.write().await;
        *w_lock = keypair;
    }
}
