// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: combine those more closely. Perhaps into a single underlying store.
// Like for persistent, on-disk, storage, what's the point of having 3 different databases?

use crate::client::key_manager::persistence::{InMemEphemeralKeys, KeyStore};
use crate::client::replies::reply_storage;
use crate::client::replies::reply_storage::ReplyStorageBackend;
use nym_credential_storage::ephemeral_storage::{
    EphemeralStorage as EphemeralCredentialStorage, EphemeralStorage,
};
use nym_credential_storage::storage::Storage as CredentialStorage;

#[cfg(not(target_arch = "wasm32"))]
use crate::client::base_client::non_wasm_helpers;
#[cfg(not(target_arch = "wasm32"))]
use crate::client::key_manager::persistence::OnDiskKeys;
#[cfg(not(target_arch = "wasm32"))]
use crate::config::{persistence::key_pathfinder::ClientKeyPathfinder, Config};
#[cfg(not(target_arch = "wasm32"))]
use crate::error::ClientCoreError;
#[cfg(not(target_arch = "wasm32"))]
use nym_credential_storage::persistent_storage::PersistentStorage as PersistentCredentialStorage;

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
use crate::client::replies::reply_storage::fs_backend;

pub trait MixnetClientStorage {
    type KeyStore: KeyStore;
    type ReplyStore: ReplyStorageBackend;
    type CredentialStore: CredentialStorage;

    // this is a TERRIBLE name...
    fn into_split(self) -> (Self::KeyStore, Self::ReplyStore, Self::CredentialStore);

    fn key_store(&self) -> &Self::KeyStore;
    fn reply_store(&self) -> &Self::ReplyStore;
    fn credential_store(&self) -> &Self::CredentialStore;
}

#[derive(Default)]
pub struct Ephemeral {
    key_store: InMemEphemeralKeys,
    reply_store: reply_storage::Empty,
    credential_store: EphemeralStorage,
}

impl Ephemeral {
    pub fn new() -> Self {
        Default::default()
    }
}

impl MixnetClientStorage for Ephemeral {
    type KeyStore = InMemEphemeralKeys;
    type ReplyStore = reply_storage::Empty;
    type CredentialStore = EphemeralCredentialStorage;

    fn into_split(self) -> (Self::KeyStore, Self::ReplyStore, Self::CredentialStore) {
        (self.key_store, self.reply_store, self.credential_store)
    }

    fn key_store(&self) -> &Self::KeyStore {
        &self.key_store
    }

    fn reply_store(&self) -> &Self::ReplyStore {
        &self.reply_store
    }

    fn credential_store(&self) -> &Self::CredentialStore {
        &self.credential_store
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct OnDiskPersistent {
    pub(crate) key_store: OnDiskKeys,
    pub(crate) reply_store: fs_backend::Backend,
    pub(crate) credential_store: PersistentCredentialStorage,
}

#[cfg(not(target_arch = "wasm32"))]
impl OnDiskPersistent {
    pub fn new(
        key_store: OnDiskKeys,
        reply_store: fs_backend::Backend,
        credential_store: PersistentCredentialStorage,
    ) -> Self {
        Self {
            key_store,
            reply_store,
            credential_store,
        }
    }

    pub async fn from_config<T>(config: &Config<T>) -> Result<Self, ClientCoreError> {
        let pathfinder = ClientKeyPathfinder::new_from_config(config);
        let key_store = OnDiskKeys::new(pathfinder);

        let reply_store = non_wasm_helpers::setup_fs_reply_surb_backend(
            config.get_reply_surb_database_path(),
            &config.get_debug_config().reply_surbs,
        )
        .await?;

        let credential_store =
            nym_credential_storage::initialise_persistent_storage(config.get_database_path()).await;

        Ok(OnDiskPersistent {
            key_store,
            reply_store,
            credential_store,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl MixnetClientStorage for OnDiskPersistent {
    type KeyStore = OnDiskKeys;
    type ReplyStore = fs_backend::Backend;
    type CredentialStore = PersistentCredentialStorage;

    fn into_split(self) -> (Self::KeyStore, Self::ReplyStore, Self::CredentialStore) {
        (self.key_store, self.reply_store, self.credential_store)
    }

    fn key_store(&self) -> &Self::KeyStore {
        &self.key_store
    }

    fn reply_store(&self) -> &Self::ReplyStore {
        &self.reply_store
    }

    fn credential_store(&self) -> &Self::CredentialStore {
        &self.credential_store
    }
}
