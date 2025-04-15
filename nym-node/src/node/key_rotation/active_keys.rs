// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::key_rotation::key::SphinxPrivateKey;
use arc_swap::access::Access;
use arc_swap::{ArcSwap, ArcSwapOption, Guard};
use nym_crypto::aes::cipher::crypto_common::rand_core::{CryptoRng, RngCore};
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ActiveSphinxKeys {
    inner: Arc<ActiveSphinxKeysInner>,
}

struct ActiveSphinxKeysInner {
    primary_key: ArcSwap<SphinxPrivateKey>,
    secondary_key: ArcSwapOption<SphinxPrivateKey>,
}

impl ActiveSphinxKeys {
    pub(crate) fn new_fresh(primary: SphinxPrivateKey) -> Self {
        ActiveSphinxKeys {
            inner: Arc::new(ActiveSphinxKeysInner {
                primary_key: ArcSwap::from_pointee(primary),
                secondary_key: Default::default(),
            }),
        }
    }

    pub(crate) fn even(&self) -> Option<impl Deref<Target = SphinxPrivateKey>> {
        let primary = self.inner.primary_key.load();
        if primary.is_even_rotation() {
            return Some(primary);
        }
        self.secondary()
    }

    pub(crate) fn odd(&self) -> Option<impl Deref<Target = SphinxPrivateKey>> {
        let primary = self.inner.primary_key.load();
        if !primary.is_even_rotation() {
            return Some(primary);
        }
        self.secondary()
    }

    pub(crate) fn primary(&self) -> impl Deref<Target = SphinxPrivateKey> {
        self.inner.primary_key.map(|k: &SphinxPrivateKey| k).load()
    }

    pub(crate) fn secondary(&self) -> Option<impl Deref<Target = SphinxPrivateKey>> {
        let guard = self.inner.secondary_key.load();
        if guard.is_none() {
            return None;
        }

        Some(SecondaryKeyGuard { guard })
    }

    // 1. generate new key
    // 2. save it in a temp file
    // 3. copy primary key file to secondary files
    // 4. set primary to secondary
    // 5. move temp file to primary
    // 6. set new key to primary
    // 7. announce it to nym-api
    fn rotate<R: RngCore + CryptoRng>(&self, rng: &mut R) {
        todo!("check rotation id");
        //
        // if self.inner.secondary_key.load().is_some() {
        //     // this should NEVER happen, but technically nothing should blow up
        //     error!("somehow our secondary key was still set during the rotation!")
        // }
        //
        // let new = x25519::KeyPair::new(rng);
        // let todo = "backup the key";
        // // we also need to announce it here...
        //
        // let old_primary = self.inner.primary_key.swap(Arc::new(new));
        // self.inner.secondary_key.store(Some(old_primary));
        //
        // // TODO: backup new key + remove old key
    }

    fn deactivate_secondary(&self) {
        self.inner.secondary_key.store(None);
    }
}

pub(crate) struct SecondaryKeyGuard {
    guard: Guard<Option<Arc<SphinxPrivateKey>>>,
}

impl Deref for SecondaryKeyGuard {
    type Target = SphinxPrivateKey;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the guard is ONLY constructed when the key is 'Some'
        #[allow(clippy::unwrap_used)]
        self.guard.as_ref().unwrap()
    }
}
