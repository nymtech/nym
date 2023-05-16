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
use crate::client::key_manager::persistence::OnDiskKeys;

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
