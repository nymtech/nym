// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::gateway_keys::{GatewayClientKeys, GatewayKeyDerivation};
use crate::client::key_manager::persistence::KeyStore;
use nym_client_core_gateways_storage::GenerationId;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_gateway_requests::shared_key::SharedSymmetricKey;
use nym_sphinx::acknowledgements::AckKey;
use rand::CryptoRng;
use std::sync::Arc;
use zeroize::ZeroizeOnDrop;

pub mod gateway_keys;
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
    identity_keypair: Arc<ed25519::KeyPair>,

    /// encryption key associated with the client instance.
    encryption_keypair: Arc<x25519::KeyPair>,

    /// key used for producing and processing acknowledgement packets.
    ack_key: Arc<AckKey>,
}

impl ClientKeys {
    /// Creates new instance of a [`ClientKeys`]
    pub fn generate_new<R>(rng: &mut R) -> Self
    where
        R: CryptoRng,
    {
        ClientKeys {
            identity_keypair: Arc::new(ed25519::KeyPair::new(rng)),
            encryption_keypair: Arc::new(x25519::KeyPair::new(rng)),
            ack_key: Arc::new(AckKey::new(rng)),
        }
    }

    pub fn from_keys(
        id_keypair: ed25519::KeyPair,
        enc_keypair: x25519::KeyPair,
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

    /// Gets an atomically reference counted pointer to [`ed25519::KeyPair`].
    pub fn identity_keypair(&self) -> Arc<ed25519::KeyPair> {
        Arc::clone(&self.identity_keypair)
    }

    /// Gets an atomically reference counted pointer to [`x25519::KeyPair`].
    pub fn encryption_keypair(&self) -> Arc<x25519::KeyPair> {
        Arc::clone(&self.encryption_keypair)
    }
    /// Gets an atomically reference counted pointer to [`AckKey`].
    pub fn ack_key(&self) -> Arc<AckKey> {
        Arc::clone(&self.ack_key)
    }

    /// The keys for a gateway: derived at the generation if one is given, otherwise the root keys.
    pub fn gateway_keys(
        &self,
        gateway_id: ed25519::PublicKey,
        generation_id: Option<GenerationId>,
    ) -> GatewayClientKeys {
        let Some(generation_id) = generation_id else {
            return GatewayClientKeys {
                identity_keypair: self.identity_keypair(),
                encryption_keypair: self.encryption_keypair(),
                ack_key: self.ack_key(),
            };
        };

        let derivation = GatewayKeyDerivation::new(
            self.identity_keypair.private_key(),
            gateway_id,
            generation_id,
        );
        GatewayClientKeys {
            identity_keypair: Arc::new(derivation.identity_keypair()),
            encryption_keypair: Arc::new(derivation.diffie_hellman_keypair()),
            ack_key: self.ack_key(),
        }
    }
}

fn _assert_keys_zeroize_on_drop() {
    fn _assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    _assert_zeroize_on_drop::<ed25519::KeyPair>();
    _assert_zeroize_on_drop::<x25519::KeyPair>();
    _assert_zeroize_on_drop::<AckKey>();
    _assert_zeroize_on_drop::<SharedSymmetricKey>();
}
