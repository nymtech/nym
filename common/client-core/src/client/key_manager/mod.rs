// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::persistence::KeyStore;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_sphinx::acknowledgements::AckKey;
use rand::{CryptoRng, RngCore};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use zeroize::ZeroizeOnDrop;

pub mod persistence;

pub enum ManagedKeys {
    Initial(KeyManagerBuilder),
    FullyDerived(KeyManager),

    // I really hate the existence of this variant, but I couldn't come up with a better way to handle
    // `Self::deal_with_gateway_key` otherwise.
    Invalidated,
}

impl Debug for ManagedKeys {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ManagedKeys::Initial(_) => write!(f, "initial"),
            ManagedKeys::FullyDerived(_) => write!(f, "fully derived"),
            ManagedKeys::Invalidated => write!(f, "invalidated"),
        }
    }
}

impl From<KeyManagerBuilder> for ManagedKeys {
    fn from(value: KeyManagerBuilder) -> Self {
        ManagedKeys::Initial(value)
    }
}

impl From<KeyManager> for ManagedKeys {
    fn from(value: KeyManager) -> Self {
        ManagedKeys::FullyDerived(value)
    }
}

impl ManagedKeys {
    pub fn is_valid(&self) -> bool {
        !matches!(self, ManagedKeys::Invalidated)
    }

    pub async fn try_load<S: KeyStore>(key_store: &S) -> Result<Self, S::StorageError> {
        Ok(ManagedKeys::FullyDerived(
            KeyManager::load_keys(key_store).await?,
        ))
    }

    pub fn generate_new<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        ManagedKeys::Initial(KeyManagerBuilder::new(rng))
    }

    pub async fn load_or_generate<R, S>(rng: &mut R, key_store: &S) -> Self
    where
        R: RngCore + CryptoRng,
        S: KeyStore,
    {
        Self::try_load(key_store)
            .await
            .unwrap_or_else(|_| Self::generate_new(rng))
    }

    pub fn identity_keypair(&self) -> Arc<identity::KeyPair> {
        match self {
            ManagedKeys::Initial(keys) => keys.identity_keypair(),
            ManagedKeys::FullyDerived(keys) => keys.identity_keypair(),
            ManagedKeys::Invalidated => unreachable!("the managed keys got invalidated"),
        }
    }

    pub fn encryption_keypair(&self) -> Arc<encryption::KeyPair> {
        match self {
            ManagedKeys::Initial(keys) => keys.encryption_keypair(),
            ManagedKeys::FullyDerived(keys) => keys.encryption_keypair(),
            ManagedKeys::Invalidated => unreachable!("the managed keys got invalidated"),
        }
    }

    pub fn ack_key(&self) -> Arc<AckKey> {
        match self {
            ManagedKeys::Initial(keys) => keys.ack_key(),
            ManagedKeys::FullyDerived(keys) => keys.ack_key(),
            ManagedKeys::Invalidated => unreachable!("the managed keys got invalidated"),
        }
    }

    pub fn must_get_gateway_shared_key(&self) -> Arc<SharedKeys> {
        self.gateway_shared_key()
            .expect("failed to extract gateway shared key")
    }

    pub fn gateway_shared_key(&self) -> Option<Arc<SharedKeys>> {
        match self {
            ManagedKeys::Initial(_) => None,
            ManagedKeys::FullyDerived(keys) => keys.gateway_shared_key(),
            ManagedKeys::Invalidated => unreachable!("the managed keys got invalidated"),
        }
    }

    pub fn identity_public_key(&self) -> &identity::PublicKey {
        match self {
            ManagedKeys::Initial(keys) => keys.identity_keypair.public_key(),
            ManagedKeys::FullyDerived(keys) => keys.identity_keypair.public_key(),
            ManagedKeys::Invalidated => unreachable!("the managed keys got invalidated"),
        }
    }

    pub fn encryption_public_key(&self) -> &encryption::PublicKey {
        match self {
            ManagedKeys::Initial(keys) => keys.encryption_keypair.public_key(),
            ManagedKeys::FullyDerived(keys) => keys.encryption_keypair.public_key(),
            ManagedKeys::Invalidated => unreachable!("the managed keys got invalidated"),
        }
    }

    pub fn ensure_gateway_key(&self, gateway_shared_key: Option<Arc<SharedKeys>>) {
        if let ManagedKeys::FullyDerived(key_manager) = &self {
            if self.gateway_shared_key().is_none() && gateway_shared_key.is_none() {
                // the key doesn't exist in either state
                return;
            }

            if gateway_shared_key.is_some() && self.gateway_shared_key().is_none()
                || gateway_shared_key.is_none() && self.gateway_shared_key().is_some()
            {
                // if one is provided whilst the other is not...
                // TODO: should this actually panic or return an error? would this branch be possible
                // under normal operation?
                panic!("inconsistent re-derived gateway key")
            }

            // here we know both keys MUST exist
            let provided = gateway_shared_key.unwrap();
            if !Arc::ptr_eq(key_manager.must_get_gateway_shared_key(), &provided)
                || *key_manager.must_get_gateway_shared_key() != provided
            {
                // this should NEVER happen thus panic here
                panic!("derived fresh gateway shared key whilst already holding one!")
            }
        }
    }

    pub async fn deal_with_gateway_key<S: KeyStore>(
        &mut self,
        gateway_shared_key: Option<Arc<SharedKeys>>,
        key_store: &S,
    ) -> Result<(), S::StorageError> {
        let key_manager = match std::mem::replace(self, ManagedKeys::Invalidated) {
            ManagedKeys::Initial(keys) => {
                let key_manager = keys.insert_maybe_gateway_shared_key(gateway_shared_key);
                key_manager.persist_keys(key_store).await?;
                key_manager
            }
            ManagedKeys::FullyDerived(key_manager) => {
                self.ensure_gateway_key(gateway_shared_key);
                key_manager
            }
            ManagedKeys::Invalidated => unreachable!("the managed keys got invalidated"),
        };

        *self = ManagedKeys::FullyDerived(key_manager);
        Ok(())
    }
}

// all of the keys really shouldn't be wrapped in `Arc`, but due to how the gateway client is currently
// constructed, changing that would require more work than what it's worth
pub struct KeyManagerBuilder {
    /// identity key associated with the client instance.
    identity_keypair: Arc<identity::KeyPair>,

    /// encryption key associated with the client instance.
    encryption_keypair: Arc<encryption::KeyPair>,

    /// key used for producing and processing acknowledgement packets.
    ack_key: Arc<AckKey>,
}

impl KeyManagerBuilder {
    /// Creates new instance of a [`KeyManager`]
    pub fn new<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        KeyManagerBuilder {
            identity_keypair: Arc::new(identity::KeyPair::new(rng)),
            encryption_keypair: Arc::new(encryption::KeyPair::new(rng)),
            ack_key: Arc::new(AckKey::new(rng)),
        }
    }

    pub fn insert_maybe_gateway_shared_key(
        self,
        gateway_shared_key: Option<Arc<SharedKeys>>,
    ) -> KeyManager {
        KeyManager {
            identity_keypair: self.identity_keypair,
            encryption_keypair: self.encryption_keypair,
            gateway_shared_key,
            ack_key: self.ack_key,
        }
    }

    pub fn identity_keypair(&self) -> Arc<identity::KeyPair> {
        Arc::clone(&self.identity_keypair)
    }

    pub fn encryption_keypair(&self) -> Arc<encryption::KeyPair> {
        Arc::clone(&self.encryption_keypair)
    }

    pub fn ack_key(&self) -> Arc<AckKey> {
        Arc::clone(&self.ack_key)
    }
}

// Note: to support key rotation in the future, all keys will require adding an extra smart pointer,
// most likely an AtomicCell, or if it doesn't work as I think it does, a Mutex. Although I think
// AtomicCell includes a Mutex implicitly if the underlying type does not work atomically.
// And I guess there will need to be some mechanism for a grace period when you can still
// use the old key after new one was issued.

// Remember that Arc<T> has Deref implementation for T
#[derive(Clone)]
pub struct KeyManager {
    /// identity key associated with the client instance.
    identity_keypair: Arc<identity::KeyPair>,

    /// encryption key associated with the client instance.
    encryption_keypair: Arc<encryption::KeyPair>,

    /// shared key derived with the gateway during "registration handshake"
    // I'm not a fan of how we broke the nice transition of `KeyManagerBuilder` -> `KeyManager`
    // by making this field optional.
    // However, it has to be optional for when we use embedded NR inside a gateway,
    // since it won't have a shared key (because why would it?)
    gateway_shared_key: Option<Arc<SharedKeys>>,

    /// key used for producing and processing acknowledgement packets.
    ack_key: Arc<AckKey>,
}

impl KeyManager {
    pub fn from_keys(
        id_keypair: identity::KeyPair,
        enc_keypair: encryption::KeyPair,
        gateway_shared_key: Option<SharedKeys>,
        ack_key: AckKey,
    ) -> Self {
        Self {
            identity_keypair: Arc::new(id_keypair),
            encryption_keypair: Arc::new(enc_keypair),
            gateway_shared_key: gateway_shared_key.map(Arc::new),
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

    fn must_get_gateway_shared_key(&self) -> &Arc<SharedKeys> {
        self.gateway_shared_key
            .as_ref()
            .expect("gateway shared key is unavailable")
    }

    pub fn uses_custom_gateway(&self) -> bool {
        self.gateway_shared_key.is_none()
    }

    /// Gets an atomically reference counted pointer to [`SharedKey`].
    pub fn gateway_shared_key(&self) -> Option<Arc<SharedKeys>> {
        self.gateway_shared_key.clone()
    }

    pub fn remove_gateway_key(self) -> KeyManagerBuilder {
        if Arc::strong_count(self.must_get_gateway_shared_key()) > 1 {
            panic!("attempted to remove gateway key whilst still holding multiple references!")
        }
        KeyManagerBuilder {
            identity_keypair: self.identity_keypair,
            encryption_keypair: self.encryption_keypair,
            ack_key: self.ack_key,
        }
    }
}

fn _assert_keys_zeroize_on_drop() {
    fn _assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    _assert_zeroize_on_drop::<identity::KeyPair>();
    _assert_zeroize_on_drop::<encryption::KeyPair>();
    _assert_zeroize_on_drop::<AckKey>();
    _assert_zeroize_on_drop::<SharedKeys>();
}
