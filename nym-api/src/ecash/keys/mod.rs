// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::VerificationKeyAuth;
use nym_dkg::Scalar;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

mod persistence;

#[derive(Clone, Debug)]
pub struct KeyPair {
    // keys: Arc<RwLock<HashMap<EpochId, nym_coconut_interface::KeyPair>>>,
    keys: Arc<RwLock<Option<KeyPairWithEpoch>>>,
    valid: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct KeyPairWithEpoch {
    pub(crate) keys: nym_compact_ecash::KeyPairAuth,
    pub(crate) issued_for_epoch: EpochId,
}

impl KeyPairWithEpoch {
    pub(crate) fn new(keys: nym_compact_ecash::KeyPairAuth, issued_for_epoch: EpochId) -> Self {
        KeyPairWithEpoch {
            keys,
            issued_for_epoch,
        }
    }

    // extract underlying secrets from the coconut's secret key.
    // the caller of this function must exercise extreme care to not misuse the data and ensuring it gets zeroized
    // `KeyPair` and `SecretKey` implement ZeroizeOnDrop; `Scalar` does not (it implements `Copy` -> important to keep in mind)
    pub(crate) fn hazmat_into_secrets(self) -> Vec<Scalar> {
        let (x, mut secrets) = self.keys.secret_key().hazmat_to_raw();

        secrets.insert(0, x);
        secrets
        // since `nym_coconut_interface::KeyPair` implements `ZeroizeOnDrop` and we took ownership of the keypair,
        // it will get zeroized after we exit this scope
    }
}

impl KeyPair {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(None)),
            valid: Arc::new(Default::default()),
        }
    }

    pub async fn take(&self) -> Option<KeyPairWithEpoch> {
        self.keys.write().await.take()
    }

    pub async fn get(&self) -> Option<RwLockReadGuard<'_, Option<KeyPairWithEpoch>>> {
        if self.is_valid() {
            Some(self.read_keys().await)
        } else {
            None
        }
    }

    pub async fn verification_key(&self) -> Option<VerificationKeyAuth> {
        self.get()
            .await?
            .as_ref()
            .map(|k| k.keys.verification_key())
    }

    pub async fn read_keys(&self) -> RwLockReadGuard<'_, Option<KeyPairWithEpoch>> {
        self.keys.read().await
    }

    pub async fn set(&self, keypair: KeyPairWithEpoch) {
        let mut w_lock = self.keys.write().await;
        *w_lock = Some(keypair);
    }

    pub fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }

    pub fn validate(&self) {
        self.valid.store(true, Ordering::SeqCst);
    }

    pub fn invalidate(&self) {
        self.valid.store(false, Ordering::SeqCst);
    }
}
