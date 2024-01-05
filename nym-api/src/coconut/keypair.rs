// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_coconut_dkg_common::types::EpochId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Clone, Debug)]
pub struct KeyPair {
    // keys: Arc<RwLock<HashMap<EpochId, nym_coconut_interface::KeyPair>>>,
    keys: Arc<RwLock<Option<(EpochId, nym_coconut_interface::KeyPair)>>>,
    // issued_for_epoch: Arc<AtomicU64>,
}

impl KeyPair {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn take(&self) -> Option<(EpochId, nym_coconut::KeyPair)> {
        self.keys.write().await.take()
    }

    pub async fn get(&self) -> RwLockReadGuard<'_, Option<nym_coconut::KeyPair>> {
        todo!()
        // self.keys.read().await
    }

    pub async fn set(&self, epoch_id: EpochId, keypair: nym_coconut_interface::KeyPair) {
        todo!()
        // let mut w_lock = self.keys.write().await;
        // *w_lock = Some(keypair);
    }

    pub async fn invalidate(&self) {
        *self.keys.write().await = None
    }
}
