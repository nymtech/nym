// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::access::{Access, MapGuard};
use arc_swap::{ArcSwap, ArcSwapOption};
use nym_crypto::asymmetric::x25519;
use rand::{CryptoRng, RngCore};
use std::sync::Arc;
use tracing::error;

pub(crate) struct SphinxKeyManager {
    primary_key: Arc<ArcSwap<x25519::KeyPair>>,
    secondary_key: Arc<ArcSwapOption<x25519::KeyPair>>,
}

type ActiveKey = ArcSwap<Arc<x25519::KeyPair>>;

impl SphinxKeyManager {
    pub(crate) fn initialise_new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let keypair = x25519::KeyPair::new(rng);

        todo!()
    }

    pub(crate) fn primary(&self) -> &x25519::PrivateKey {
        let foo: MapGuard<_, _, _, _> = self
            .primary_key
            .map(|primary: &x25519::KeyPair| primary.private_key())
            .load();

        // ArcSwap::map(self.primary_key.load(), |primary| primary.as_ref())
        todo!()
    }

    pub(crate) fn secondary(&self) -> &x25519::PrivateKey {
        self.secondary_key.load().as_ref().map(|k| k.private_key())
    }

    // 1. generate new key
    // 2. save it in a temp file
    // 3. copy primary key file to secondary files
    // 4. set primary to secondary
    // 5. move temp file to primary
    // 6. set new key to primary
    // 7. announce it to nym-api
    fn rotate<R: RngCore + CryptoRng>(&self, rng: &mut R) {
        if self.secondary_key.load().is_some() {
            // this should NEVER happen, but technically nothing should blow up
            error!("somehow our secondary key was still set during the rotation!")
        }

        let new = x25519::KeyPair::new(rng);
        let todo = "backup the key";
        // we also need to announce it here...

        let old_primary = self.primary_key.swap(Arc::new(new));
        self.secondary_key.store(Some(old_primary));

        // TODO: backup new key + remove old key
    }

    fn deactivate_secondary(&self) {
        self.secondary_key.store(None);
    }
}
