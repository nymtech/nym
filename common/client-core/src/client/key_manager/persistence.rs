// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::KeyManager;
use async_trait::async_trait;
use std::error::Error;

#[cfg(not(target_arch = "wasm32"))]
use crate::config::persistence::key_pathfinder::ClientKeyPathfinder;
#[cfg(not(target_arch = "wasm32"))]
use nym_crypto::asymmetric::{encryption, identity};
#[cfg(not(target_arch = "wasm32"))]
use nym_gateway_requests::registration::handshake::SharedKeys;
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
pub struct OnDiskKeys<'a> {
    pathfinder: &'a ClientKeyPathfinder,
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> From<&'a ClientKeyPathfinder> for OnDiskKeys<'a> {
    fn from(pathfinder: &'a ClientKeyPathfinder) -> Self {
        OnDiskKeys { pathfinder }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<'a> OnDiskKeys<'a> {
    pub fn new(pathfinder: &'a ClientKeyPathfinder) -> Self {
        OnDiskKeys { pathfinder }
    }

    fn load_keys(&self) -> Result<KeyManager, std::io::Error> {
        let identity_keypair: identity::KeyPair =
            nym_pemstore::load_keypair(&self.pathfinder.identity_key_pair_path())?;
        let encryption_keypair: encryption::KeyPair =
            nym_pemstore::load_keypair(&self.pathfinder.encryption_key_pair_path())?;

        let ack_key: AckKey = nym_pemstore::load_key(self.pathfinder.ack_key())?;
        let gateway_shared_key: SharedKeys =
            nym_pemstore::load_key(self.pathfinder.gateway_shared_key())?;

        Ok(KeyManager::from_keys(
            identity_keypair,
            encryption_keypair,
            gateway_shared_key,
            ack_key,
        ))
    }

    fn store_keys(&self, keys: &KeyManager) -> Result<(), std::io::Error> {
        nym_pemstore::store_keypair(
            keys.identity_keypair.as_ref(),
            &self.pathfinder.identity_key_pair_path(),
        )?;
        nym_pemstore::store_keypair(
            keys.encryption_keypair.as_ref(),
            &self.pathfinder.encryption_key_pair_path(),
        )?;

        nym_pemstore::store_key(
            keys.gateway_shared_key.as_ref(),
            self.pathfinder.gateway_shared_key(),
        )?;
        nym_pemstore::store_key(keys.ack_key.as_ref(), self.pathfinder.ack_key())?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<'a> KeyStore for OnDiskKeys<'a> {
    type StorageError = std::io::Error;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        self.load_keys()
    }

    async fn store_keys(&self, keys: &KeyManager) -> Result<(), Self::StorageError> {
        self.store_keys(keys)
    }
}

pub struct InMemEphemeralKeys;

#[derive(Debug, thiserror::Error)]
#[error("ephemeral keys can't be loaded from storage")]
pub struct EphemeralKeysError;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl KeyStore for InMemEphemeralKeys {
    type StorageError = EphemeralKeysError;

    async fn load_keys(&self) -> Result<KeyManager, Self::StorageError> {
        Err(EphemeralKeysError)
    }

    async fn store_keys(&self, _keys: &KeyManager) -> Result<(), Self::StorageError> {
        Ok(())
    }
}
