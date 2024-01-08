// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

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

    pub async fn take(&self) -> Option<nym_coconut::KeyPair> {
        self.inner.write().await.take()
    }

    pub async fn get(&self) -> RwLockReadGuard<'_, Option<nym_coconut::KeyPair>> {
        self.inner.read().await
    }

    pub async fn set(&self, keypair: Option<nym_coconut_interface::KeyPair>) {
        let mut w_lock = self.inner.write().await;
        *w_lock = keypair;
    }
}
