// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Clone, Debug)]
pub struct KeyPair {
    inner: Arc<RwLock<Option<coconut_interface::KeyPair>>>,
}

impl KeyPair {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get(&self) -> RwLockReadGuard<'_, Option<nymcoconut::KeyPair>> {
        self.inner.read().await
    }

    pub async fn set(&self, keypair: coconut_interface::KeyPair) {
        let mut w_lock = self.inner.write().await;
        *w_lock = Some(keypair);
    }
}
