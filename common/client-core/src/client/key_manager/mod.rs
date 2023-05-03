// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::persistence::KeyStore;
use nym_crypto::asymmetric::{encryption, identity};
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_sphinx::acknowledgements::AckKey;
use rand::{CryptoRng, RngCore};
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

    pub async fn must_load<S: KeyStore>(key_store: &S) -> Result<Self, S::StorageError> {
        Ok(ManagedKeys::FullyDerived(
            KeyManager::load_keys(key_store).await?,
        ))
    }

    pub async fn load_or_generate<R, S>(rng: &mut R, key_store: &S) -> Self
    where
        R: RngCore + CryptoRng,
        S: KeyStore,
    {
        if let Ok(loaded) = KeyManager::load_keys(key_store).await {
            ManagedKeys::FullyDerived(loaded)
        } else {
            ManagedKeys::Initial(KeyManagerBuilder::new(rng))
        }
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

    pub fn gateway_shared_key(&self) -> Option<Arc<SharedKeys>> {
        match self {
            ManagedKeys::Initial(_) => None,
            ManagedKeys::FullyDerived(keys) => Some(keys.gateway_shared_key()),
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

    pub async fn deal_with_gateway_key<S: KeyStore>(
        &mut self,
        gateway_shared_key: Arc<SharedKeys>,
        key_store: &S,
    ) -> Result<(), S::StorageError> {
        let key_manager = match std::mem::replace(self, ManagedKeys::Invalidated) {
            ManagedKeys::Initial(keys) => {
                let key_manager = keys.insert_gateway_shared_key(gateway_shared_key);
                key_manager.persist_keys(key_store).await?;
                key_manager
            }
            ManagedKeys::FullyDerived(key_manager) => {
                if !Arc::ptr_eq(&key_manager.gateway_shared_key, &gateway_shared_key)
                    || key_manager.gateway_shared_key != gateway_shared_key
                {
                    // this should NEVER happen thus panic here
                    panic!("derived fresh gateway shared key whilst already holding one!")
                }
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

    pub fn insert_gateway_shared_key(self, gateway_shared_key: Arc<SharedKeys>) -> KeyManager {
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
    gateway_shared_key: Arc<SharedKeys>,

    /// key used for producing and processing acknowledgement packets.
    ack_key: Arc<AckKey>,
}

// The expected flow of a KeyManager "lifetime" is as follows:
/*
   1. ::new() is called during client-init
   2. after gateway registration is completed [in init] ::insert_gateway_shared_key() is called
   3. ::store_keys() is called before init finishes execution.
   4. ::load_keys() is called at the beginning of each subsequent client-run
   5. [not implemented] ::rotate_keys() is called periodically during client-run I presume?
*/

impl KeyManager {
    // /// Creates new instance of a [`KeyManager`]
    // pub fn new<R>(rng: &mut R) -> Self
    // where
    //     R: RngCore + CryptoRng,
    // {
    //     KeyManager {
    //         identity_keypair: Arc::new(identity::KeyPair::new(rng)),
    //         encryption_keypair: Arc::new(encryption::KeyPair::new(rng)),
    //         gateway_shared_key: None,
    //         ack_key: Arc::new(AckKey::new(rng)),
    //     }
    // }

    pub fn from_keys(
        id_keypair: identity::KeyPair,
        enc_keypair: encryption::KeyPair,
        gateway_shared_key: SharedKeys,
        ack_key: AckKey,
    ) -> Self {
        Self {
            identity_keypair: Arc::new(id_keypair),
            encryption_keypair: Arc::new(enc_keypair),
            gateway_shared_key: Arc::new(gateway_shared_key),
            ack_key: Arc::new(ack_key),
        }
    }

    pub async fn load_keys<S: KeyStore>(store: &S) -> Result<Self, S::StorageError> {
        store.load_keys().await
    }

    pub async fn persist_keys<S: KeyStore>(&self, store: &S) -> Result<(), S::StorageError> {
        store.store_keys(self).await
    }

    //
    // /// Loads previously stored client keys from the disk.
    // fn load_client_keys(client_pathfinder: &ClientKeyPathfinder) -> io::Result<Self> {
    //     let identity_keypair: identity::KeyPair =
    //         nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
    //             client_pathfinder.private_identity_key().to_owned(),
    //             client_pathfinder.public_identity_key().to_owned(),
    //         ))?;
    //     let encryption_keypair: encryption::KeyPair =
    //         nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
    //             client_pathfinder.private_encryption_key().to_owned(),
    //             client_pathfinder.public_encryption_key().to_owned(),
    //         ))?;
    //
    //     let ack_key: AckKey = nym_pemstore::load_key(client_pathfinder.ack_key())?;
    //
    //     Ok(KeyManager {
    //         identity_keypair: Arc::new(identity_keypair),
    //         encryption_keypair: Arc::new(encryption_keypair),
    //         gateway_shared_key: None,
    //         ack_key: Arc::new(ack_key),
    //     })
    // }
    //
    // /// Loads previously stored keys from the disk. Fails if not all, including the shared gateway
    // /// key, is available.
    // pub fn load_keys_from_disk(client_pathfinder: &ClientKeyPathfinder) -> io::Result<Self> {
    //     let mut key_manager = Self::load_client_keys(client_pathfinder)?;
    //
    //     let gateway_shared_key: SharedKeys =
    //         nym_pemstore::load_key(client_pathfinder.gateway_shared_key())?;
    //
    //     key_manager.gateway_shared_key = Some(Arc::new(gateway_shared_key));
    //
    //     Ok(key_manager)
    // }
    //
    // /// Loads previously stored keys from the disk. Fails if client keys are not available, but the
    // /// shared gateway key is optional.
    // pub fn load_keys_from_disk_but_gateway_is_optional(
    //     client_pathfinder: &ClientKeyPathfinder,
    // ) -> io::Result<Self> {
    //     let mut key_manager = Self::load_client_keys(client_pathfinder)?;
    //
    //     let gateway_shared_key: Result<SharedKeys, io::Error> =
    //         nym_pemstore::load_key(client_pathfinder.gateway_shared_key());
    //
    //     // It's ok if the gateway key was not found
    //     let gateway_shared_key = match gateway_shared_key {
    //         Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
    //         Err(err) => Err(err),
    //         Ok(key) => Ok(Some(key)),
    //     }?;
    //
    //     key_manager.gateway_shared_key = gateway_shared_key.map(Arc::new);
    //
    //     Ok(key_manager)
    // }
    //
    // /// Stores all available keys on the disk.
    // // While perhaps there is no much point in storing the `AckKey` on the disk,
    // // it is done so for the consistency sake so that you wouldn't require an rng instance
    // // during `load_keys` to generate the said key.
    // pub fn store_keys_on_disk(&self, client_pathfinder: &ClientKeyPathfinder) -> io::Result<()> {
    //     nym_pemstore::store_keypair(
    //         self.identity_keypair.as_ref(),
    //         &nym_pemstore::KeyPairPath::new(
    //             client_pathfinder.private_identity_key().to_owned(),
    //             client_pathfinder.public_identity_key().to_owned(),
    //         ),
    //     )?;
    //     nym_pemstore::store_keypair(
    //         self.encryption_keypair.as_ref(),
    //         &nym_pemstore::KeyPairPath::new(
    //             client_pathfinder.private_encryption_key().to_owned(),
    //             client_pathfinder.public_encryption_key().to_owned(),
    //         ),
    //     )?;
    //
    //     nym_pemstore::store_key(self.ack_key.as_ref(), client_pathfinder.ack_key())?;
    //
    //     match self.gateway_shared_key.as_ref() {
    //         None => debug!("No gateway shared key available to store!"),
    //         Some(gate_key) => {
    //             nym_pemstore::store_key(gate_key.as_ref(), client_pathfinder.gateway_shared_key())?
    //         }
    //     }
    //
    //     Ok(())
    // }
    //
    // pub fn store_gateway_key_on_disk(
    //     &self,
    //     client_pathfinder: &ClientKeyPathfinder,
    // ) -> io::Result<()> {
    //     match self.gateway_shared_key.as_ref() {
    //         None => {
    //             return Err(io::Error::new(
    //                 io::ErrorKind::Other,
    //                 "trying to store a non-existing key",
    //             ))
    //         }
    //         Some(gate_key) => {
    //             nym_pemstore::store_key(gate_key.as_ref(), client_pathfinder.gateway_shared_key())?
    //         }
    //     }
    //
    //     Ok(())
    // }

    /// Overwrite the existing identity keypair
    #[deprecated]
    pub fn set_identity_keypair(&mut self, id_keypair: identity::KeyPair) {
        self.identity_keypair = Arc::new(id_keypair);
    }

    /// Gets an atomically reference counted pointer to [`identity::KeyPair`].
    pub fn identity_keypair(&self) -> Arc<identity::KeyPair> {
        Arc::clone(&self.identity_keypair)
    }

    /// Overwrite the existing encryption keypair
    #[deprecated]
    pub fn set_encryption_keypair(&mut self, enc_keypair: encryption::KeyPair) {
        self.encryption_keypair = Arc::new(enc_keypair);
    }

    /// Gets an atomically reference counted pointer to [`encryption::KeyPair`].
    pub fn encryption_keypair(&self) -> Arc<encryption::KeyPair> {
        Arc::clone(&self.encryption_keypair)
    }

    /// Overwrite the existing ack key
    #[deprecated]
    pub fn set_ack_key(&mut self, ack_key: AckKey) {
        self.ack_key = Arc::new(ack_key);
    }

    /// Gets an atomically reference counted pointer to [`AckKey`].
    pub fn ack_key(&self) -> Arc<AckKey> {
        Arc::clone(&self.ack_key)
    }

    // /// After shared key with the gateway is derived, puts its ownership to this instance of a [`KeyManager`].
    // pub fn insert_gateway_shared_key(&mut self, gateway_shared_key: Arc<SharedKeys>) {
    //     self.gateway_shared_key = Some(gateway_shared_key)
    // }

    /// Gets an atomically reference counted pointer to [`SharedKey`].
    pub fn gateway_shared_key(&self) -> Arc<SharedKeys> {
        Arc::clone(&self.gateway_shared_key)
    }

    // pub fn is_gateway_key_set(&self) -> bool {
    //     self.gateway_shared_key.is_some()
    // }
}

fn _assert_keys_zeroize_on_drop() {
    fn _assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    _assert_zeroize_on_drop::<identity::KeyPair>();
    _assert_zeroize_on_drop::<encryption::KeyPair>();
    _assert_zeroize_on_drop::<AckKey>();
    _assert_zeroize_on_drop::<SharedKeys>();
}
