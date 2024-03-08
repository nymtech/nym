// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: combine those more closely. Perhaps into a single underlying store.
// Like for persistent, on-disk, storage, what's the point of having 3 different databases?

use crate::client::key_manager::persistence::{InMemEphemeralKeys, KeyStore};
use crate::client::replies::reply_storage;
use crate::client::replies::reply_storage::ReplyStorageBackend;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_credential_storage::storage::Storage as CredentialStorage;

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "fs-surb-storage",
    feature = "fs-gateways-storage"
))]
use crate::{
    client::{
        base_client::non_wasm_helpers, key_manager::persistence::OnDiskKeys,
        replies::reply_storage::fs_backend,
    },
    config::{self, disk_persistence::CommonClientPaths},
    error::ClientCoreError,
};
#[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
use nym_credential_storage::persistent_storage::PersistentStorage as PersistentCredentialStorage;

pub use nym_client_core_gateways_storage::{
    ActiveGateway, BadGateway, CustomGatewayDetails, GatewayDetails, GatewayRegistration,
    GatewayType, GatewaysDetailsStore, InMemGatewaysDetails, RegisteredGateway,
    RemoteGatewayDetails,
};

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-gateways-storage"))]
pub use nym_client_core_gateways_storage::{OnDiskGatewaysDetails, StorageError};

pub mod helpers;

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "fs-surb-storage",
    feature = "fs-gateways-storage"
))]
pub mod migration_helpers;

// TODO: ideally this should be changed into
// `MixnetClientStorage: KeyStore + ReplyStorageBackend + CredentialStorage + GatewaysDetailsStore`
pub trait MixnetClientStorage {
    type KeyStore: KeyStore;
    type ReplyStore: ReplyStorageBackend;
    type CredentialStore: CredentialStorage;
    type GatewaysDetailsStore: GatewaysDetailsStore;

    fn into_runtime_stores(self) -> (Self::ReplyStore, Self::CredentialStore);

    fn key_store(&self) -> &Self::KeyStore;
    fn reply_store(&self) -> &Self::ReplyStore;
    fn credential_store(&self) -> &Self::CredentialStore;
    fn gateway_details_store(&self) -> &Self::GatewaysDetailsStore;
}

#[derive(Default)]
pub struct Ephemeral {
    key_store: InMemEphemeralKeys,
    reply_store: reply_storage::Empty,
    credential_store: EphemeralCredentialStorage,
    gateway_details_store: InMemGatewaysDetails,
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
    type GatewaysDetailsStore = InMemGatewaysDetails;

    fn into_runtime_stores(self) -> (Self::ReplyStore, Self::CredentialStore) {
        (self.reply_store, self.credential_store)
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

    fn gateway_details_store(&self) -> &Self::GatewaysDetailsStore {
        &self.gateway_details_store
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "fs-surb-storage",
    feature = "fs-gateways-storage"
))]
pub struct OnDiskPersistent {
    pub(crate) key_store: OnDiskKeys,
    pub(crate) reply_store: fs_backend::Backend,
    pub(crate) credential_store: PersistentCredentialStorage,
    pub(crate) gateway_details_store: OnDiskGatewaysDetails,
}

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "fs-surb-storage",
    feature = "fs-gateways-storage"
))]
impl OnDiskPersistent {
    pub fn new(
        key_store: OnDiskKeys,
        reply_store: fs_backend::Backend,
        credential_store: PersistentCredentialStorage,
        gateway_details_store: OnDiskGatewaysDetails,
    ) -> Self {
        Self {
            key_store,
            reply_store,
            credential_store,
            gateway_details_store,
        }
    }

    pub async fn from_paths(
        paths: CommonClientPaths,
        debug_config: &config::DebugConfig,
    ) -> Result<Self, ClientCoreError> {
        let key_store = OnDiskKeys::new(paths.keys);

        let reply_store = non_wasm_helpers::setup_fs_reply_surb_backend(
            paths.reply_surb_database,
            &debug_config.reply_surbs,
        )
        .await?;

        let credential_store =
            nym_credential_storage::initialise_persistent_storage(paths.credentials_database).await;

        let gateway_details_store =
            non_wasm_helpers::setup_fs_gateways_storage(paths.gateway_registrations).await?;

        Ok(OnDiskPersistent {
            key_store,
            reply_store,
            credential_store,
            gateway_details_store,
        })
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "fs-surb-storage",
    feature = "fs-gateways-storage"
))]
impl MixnetClientStorage for OnDiskPersistent {
    type KeyStore = OnDiskKeys;
    type ReplyStore = fs_backend::Backend;
    type CredentialStore = PersistentCredentialStorage;
    type GatewaysDetailsStore = OnDiskGatewaysDetails;

    fn into_runtime_stores(self) -> (Self::ReplyStore, Self::CredentialStore) {
        (self.reply_store, self.credential_store)
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

    fn gateway_details_store(&self) -> &Self::GatewaysDetailsStore {
        &self.gateway_details_store
    }
}
