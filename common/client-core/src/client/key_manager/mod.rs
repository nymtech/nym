// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::persistence::KeyStore;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_requests::shared_key::{LegacySharedKeys, SharedGatewayKey, SharedSymmetricKey};
use nym_sphinx::acknowledgements::AckKey;
use rand::{CryptoRng, RngCore};
use std::sync::Arc;
use zeroize::ZeroizeOnDrop;

pub mod persistence;

// Note: to support key rotation in the future, all keys will require adding an extra smart pointer,
// most likely an AtomicCell, or if it doesn't work as I think it does, a Mutex. Although I think
// AtomicCell includes a Mutex implicitly if the underlying type does not work atomically.
// And I guess there will need to be some mechanism for a grace period when you can still
// use the old key after new one was issued.

// Remember that Arc<T> has Deref implementation for T
#[derive(Clone)]
pub struct ClientKeys {
    /// identity key associated with the client instance.
    identity_keypair: Arc<identity::KeyPair>,

    /// encryption key associated with the client instance.
    encryption_keypair: Arc<encryption::KeyPair>,

    /// key used for producing and processing acknowledgement packets.
    ack_key: Arc<AckKey>,
}

impl ClientKeys {
    /// Creates new instance of a [`ClientKeys`]
    pub fn generate_new<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        ClientKeys {
            identity_keypair: Arc::new(identity::KeyPair::new(rng)),
            encryption_keypair: Arc::new(encryption::KeyPair::new(rng)),
            ack_key: Arc::new(AckKey::new(rng)),
        }
    }

    pub fn from_keys(
        id_keypair: identity::KeyPair,
        enc_keypair: encryption::KeyPair,
        ack_key: AckKey,
    ) -> Self {
        Self {
            identity_keypair: Arc::new(id_keypair),
            encryption_keypair: Arc::new(enc_keypair),
            ack_key: Arc::new(ack_key),
        }
    }

    pub async fn load_keys<S: KeyStore>(store: &S) -> Result<Self, S::StorageError> {
        store.load_keys().await
    }

    pub async fn persist_keys<S: KeyStore>(&self, store: &S) -> Result<(), S::StorageError> {
        store.store_keys(self).await
    }

    /// Gets an atomically reference counted pointer to [`identity::KeyPair`].
    pub fn identity_keypair(&self) -> Arc<identity::KeyPair> {
        Arc::clone(&self.identity_keypair)
    }

    /// Gets an atomically reference counted pointer to [`encryption::KeyPair`].
    pub fn encryption_keypair(&self) -> Arc<encryption::KeyPair> {
        Arc::clone(&self.encryption_keypair)
    }
    /// Gets an atomically reference counted pointer to [`AckKey`].
    pub fn ack_key(&self) -> Arc<AckKey> {
        Arc::clone(&self.ack_key)
    }
}

fn _assert_keys_zeroize_on_drop() {
    fn _assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    _assert_zeroize_on_drop::<identity::KeyPair>();
    _assert_zeroize_on_drop::<encryption::KeyPair>();
    _assert_zeroize_on_drop::<AckKey>();
    _assert_zeroize_on_drop::<LegacySharedKeys>();
    _assert_zeroize_on_drop::<SharedSymmetricKey>();
    _assert_zeroize_on_drop::<SharedGatewayKey>();
}
