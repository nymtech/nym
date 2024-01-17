// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod persistence;

use nym_coconut_dkg_common::types::EpochId;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Clone, Debug)]
pub struct KeyPair {
    // keys: Arc<RwLock<HashMap<EpochId, nym_coconut_interface::KeyPair>>>,
    keys: Arc<RwLock<Option<KeyPairWithEpoch>>>,
}

#[derive(Debug)]
pub struct KeyPairWithEpoch {
    pub(crate) keys: nym_coconut_interface::KeyPair,
    pub(crate) issued_for_epoch: EpochId,
}

impl KeyPair {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn take(&self) -> Option<KeyPairWithEpoch> {
        self.keys.write().await.take()
    }

    pub async fn get(&self) -> RwLockReadGuard<'_, Option<KeyPairWithEpoch>> {
        todo!()
        // self.keys.read().await
    }

    pub async fn set(&self, epoch_id: EpochId, keypair: nym_coconut_interface::KeyPair) {
        todo!()
        // let mut w_lock = self.keys.write().await;
        // *w_lock = Some(keypair);
    }

    #[deprecated]
    pub async fn invalidate(&self) {
        *self.keys.write().await = None
    }
}
