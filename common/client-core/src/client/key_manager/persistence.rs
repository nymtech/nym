// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::KeyManager;
use async_trait::async_trait;
use std::error::Error;
use tokio::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use crate::config::disk_persistence::keys_paths::ClientKeysPaths;
#[cfg(not(target_arch = "wasm32"))]
use nym_crypto::asymmetric::{encryption, identity};
#[cfg(not(target_arch = "wasm32"))]
use nym_gateway_requests::registration::handshake::SharedKeys;
#[cfg(not(target_arch = "wasm32"))]
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
#[cfg(not(target_arch = "wasm32"))]
use nym_pemstore::KeyPairPath;
#[cfg(not(target_arch = "wasm32"))]
use nym_sphinx::acknowledgements::AckKey;

// we have to define it as an async trait since wasm storage is async
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait KeyStore {
    type StorageError: Error;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError>;

    async fn store_keys(&self, keys: &KeyManager) -> Result<(), Self::StorageError>;
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, thiserror::Error)]
pub enum OnDiskKeysError {
    #[error("failed to load {keys} keys from {:?} (private key) and {:?} (public key): {err}", .paths.private_key_path, .paths.public_key_path)]
    KeyPairLoadFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: std::io::Error,
    },

    #[error("failed to store {keys} keys to {:?} (private key) and {:?} (public key): {err}", .paths.private_key_path, .paths.public_key_path)]
    KeyPairStoreFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: std::io::Error,
    },

    #[error("failed to load {key} key from {path}: {err}")]
    KeyLoadFailure {
        key: String,
        path: String,
        #[source]
        err: std::io::Error,
    },

    #[error("failed to store {key} key to {path}: {err}")]
    KeyStoreFailure {
        key: String,
        path: String,
        #[source]
        err: std::io::Error,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub struct OnDiskKeys {
    paths: ClientKeysPaths,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<ClientKeysPaths> for OnDiskKeys {
    fn from(paths: ClientKeysPaths) -> Self {
        OnDiskKeys { paths }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl OnDiskKeys {
    pub fn new(paths: ClientKeysPaths) -> Self {
        OnDiskKeys { paths }
    }

    #[doc(hidden)]
    pub fn load_encryption_keypair(&self) -> Result<encryption::KeyPair, OnDiskKeysError> {
        let encryption_paths = self.paths.encryption_key_pair_path();
        self.load_keypair(encryption_paths, "encryption")
    }

    #[doc(hidden)]
    pub fn load_identity_keypair(&self) -> Result<identity::KeyPair, OnDiskKeysError> {
        let identity_paths = self.paths.identity_key_pair_path();
        self.load_keypair(identity_paths, "identity")
    }

    fn load_key<T: PemStorableKey>(
        &self,
        path: &std::path::Path,
        name: impl Into<String>,
    ) -> Result<T, OnDiskKeysError> {
        nym_pemstore::load_key(path).map_err(|err| OnDiskKeysError::KeyLoadFailure {
            key: name.into(),
            path: path.to_str().map(|s| s.to_owned()).unwrap_or_default(),
            err,
        })
    }

    fn load_keypair<T: PemStorableKeyPair>(
        &self,
        paths: KeyPairPath,
        name: impl Into<String>,
    ) -> Result<T, OnDiskKeysError> {
        nym_pemstore::load_keypair(&paths).map_err(|err| OnDiskKeysError::KeyPairLoadFailure {
            keys: name.into(),
            paths,
            err,
        })
    }

    fn store_key<T: PemStorableKey>(
        &self,
        key: &T,
        path: &std::path::Path,
        name: impl Into<String>,
    ) -> Result<(), OnDiskKeysError> {
        nym_pemstore::store_key(key, path).map_err(|err| OnDiskKeysError::KeyStoreFailure {
            key: name.into(),
            path: path.to_str().map(|s| s.to_owned()).unwrap_or_default(),
            err,
        })
    }

    fn store_keypair<T: PemStorableKeyPair>(
        &self,
        keys: &T,
        paths: KeyPairPath,
        name: impl Into<String>,
    ) -> Result<(), OnDiskKeysError> {
        nym_pemstore::store_keypair(keys, &paths).map_err(|err| {
            OnDiskKeysError::KeyPairStoreFailure {
                keys: name.into(),
                paths,
                err,
            }
        })
    }

    fn load_keys(&self) -> Result<KeyManager, OnDiskKeysError> {
        let identity_keypair = self.load_identity_keypair()?;
        let encryption_keypair = self.load_encryption_keypair()?;
        let ack_key: AckKey = self.load_key(self.paths.ack_key(), "ack key")?;

        Ok(KeyManager::from_keys(
            identity_keypair,
            encryption_keypair,
            ack_key,
        ))
    }

    fn store_keys(&self, keys: &KeyManager) -> Result<(), OnDiskKeysError> {
        let identity_paths = self.paths.identity_key_pair_path();
        let encryption_paths = self.paths.encryption_key_pair_path();

        self.store_keypair(
            keys.identity_keypair.as_ref(),
            identity_paths,
            "identity keys",
        )?;
        self.store_keypair(
            keys.encryption_keypair.as_ref(),
            encryption_paths,
            "encryption keys",
        )?;

        self.store_key(keys.ack_key.as_ref(), self.paths.ack_key(), "ack key")?;

        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl KeyStore for OnDiskKeys {
    type StorageError = OnDiskKeysError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        self.load_keys()
    }

    async fn store_keys(&self, keys: &KeyManager) -> Result<(), Self::StorageError> {
        self.store_keys(keys)
    }
}

#[derive(Default)]
pub struct InMemEphemeralKeys {
    keys: Mutex<Option<KeyManager>>,
}

#[derive(Debug, thiserror::Error)]
#[error("old ephemeral keys can't be loaded from storage")]
pub struct EphemeralKeysError;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl KeyStore for InMemEphemeralKeys {
    type StorageError = EphemeralKeysError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        self.keys.lock().await.clone().ok_or(EphemeralKeysError)
    }

    async fn store_keys(&self, keys: &KeyManager) -> Result<(), Self::StorageError> {
        *self.keys.lock().await = Some(keys.clone());
        Ok(())
    }
}
