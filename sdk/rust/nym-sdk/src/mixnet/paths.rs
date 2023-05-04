// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};
use nym_client_core::client::base_client::{non_wasm_helpers, storage};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::client::replies::reply_storage::fs_backend;
use nym_client_core::config;
use nym_client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use nym_credential_storage::persistent_storage::PersistentStorage as PersistentCredentialStorage;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum KeyMode {
    /// Use existing key files if they exists, otherwise create new ones.
    Keep,
    /// Create new keys, overwriting any potential previously existing keys.
    Overwrite,
}

impl KeyMode {
    pub(crate) fn is_keep(&self) -> bool {
        matches!(self, KeyMode::Keep)
    }
}

pub enum GatewayKeyMode {
    /// Keep shared gateway key if found, otherwise create a new one.
    Keep,
    /// Create a new shared key and overwrite any potential existing one.
    Overwrite,
}

impl GatewayKeyMode {
    pub(crate) fn is_keep(&self) -> bool {
        matches!(self, GatewayKeyMode::Keep)
    }
}

/// Set of storage paths that the client will use if it is setup to persist keys, credentials, and
/// reply-SURBs.
#[derive(Clone, Debug)]
pub struct StoragePaths {
    // /// Determines how to handle existing key files found.
    // pub operating_mode: KeyMode,
    /// Client private identity key
    pub private_identity: PathBuf,
    /// Client public identity key
    pub public_identity: PathBuf,

    /// Client private encryption key
    pub private_encryption: PathBuf,
    /// Client public encryption key
    pub public_encryption: PathBuf,

    /// Key for handling acks
    pub ack_key: PathBuf,

    /// Key setup after authenticating with a gateway
    pub gateway_shared_key: PathBuf,

    /// The key isn't much use without knowing which entity it refers to.
    pub gateway_endpoint_config: PathBuf,

    /// The database containing credentials
    pub credential_database_path: PathBuf,

    /// The database storing reply surbs in-between sessions
    pub reply_surb_database_path: PathBuf,
}

impl StoragePaths {
    /// Create a set of storage paths from a given directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if it is passed a path to an existing file instead of a
    /// directory.
    pub fn new_from_dir(operating_mode: KeyMode, dir: &Path) -> Result<Self> {
        if dir.is_file() {
            return Err(Error::ExpectedDirectory(dir.to_owned()));
        }

        Ok(Self {
            // These filenames were chosen to match the ones we use in `nym-client`. Consider
            // changing the defaults
            // operating_mode,
            private_identity: dir.join("private_identity.pem"),
            public_identity: dir.join("public_identity.pem"),
            private_encryption: dir.join("private_encryption.pem"),
            public_encryption: dir.join("public_encryption.pem"),
            ack_key: dir.join("ack_key.pem"),
            gateway_shared_key: dir.join("gateway_shared.pem"),
            gateway_endpoint_config: dir.join("gateway_endpoint_config.toml"),
            credential_database_path: dir.join("db.sqlite"),
            reply_surb_database_path: dir.join("persistent_reply_store.sqlite"),
        })
    }

    #[deprecated(note = "add docs")]
    pub async fn initialise_default_persistent_storage(
        &self,
    ) -> Result<storage::OnDiskPersistent, Error> {
        Ok(storage::OnDiskPersistent::new(
            self.on_disk_key_storage_spec(),
            self.default_persistent_fs_reply_backend().await?,
            self.persistent_credential_storage().await?,
        ))
    }

    #[deprecated(note = "add docs")]
    pub async fn initialise_persistent_storage(
        &self,
        config: &config::DebugConfig,
    ) -> Result<storage::OnDiskPersistent, Error> {
        Ok(storage::OnDiskPersistent::new(
            self.on_disk_key_storage_spec(),
            self.persistent_fs_reply_backend(&config.reply_surbs)
                .await?,
            self.persistent_credential_storage().await?,
        ))
    }

    #[deprecated(note = "add docs")]
    pub async fn persistent_credential_storage(
        &self,
    ) -> Result<PersistentCredentialStorage, Error> {
        PersistentCredentialStorage::init(&self.credential_database_path)
            .await
            .map_err(|source| Error::CredentialStorageError {
                source: Box::new(source),
            })
    }

    #[deprecated(note = "add docs")]
    pub async fn default_persistent_fs_reply_backend(&self) -> Result<fs_backend::Backend, Error> {
        self.persistent_fs_reply_backend(&Default::default()).await
    }

    pub async fn persistent_fs_reply_backend(
        &self,
        surb_config: &config::ReplySurbs,
    ) -> Result<fs_backend::Backend, Error> {
        Ok(non_wasm_helpers::setup_fs_reply_surb_backend(
            &self.reply_surb_database_path,
            surb_config,
        )
        .await?)
    }

    #[deprecated(note = "add docs")]
    pub fn on_disk_key_storage_spec(&self) -> OnDiskKeys {
        OnDiskKeys::new(self.client_key_pathfinder())
    }

    #[deprecated(note = "add docs")]
    pub fn client_key_pathfinder(&self) -> ClientKeyPathfinder {
        ClientKeyPathfinder {
            identity_private_key: self.private_identity.clone(),
            identity_public_key: self.public_identity.clone(),
            encryption_private_key: self.private_encryption.clone(),
            encryption_public_key: self.public_encryption.clone(),
            gateway_shared_key: self.gateway_shared_key.clone(),
            ack_key: self.ack_key.clone(),
        }
    }
}

impl From<StoragePaths> for ClientKeyPathfinder {
    fn from(paths: StoragePaths) -> Self {
        paths.client_key_pathfinder()
    }
}

impl<T> From<&nym_client_core::config::Config<T>> for StoragePaths {
    fn from(value: &nym_client_core::config::Config<T>) -> Self {
        Self {
            // operating_mode: KeyMode::Keep,
            private_identity: value.get_private_identity_key_file(),
            public_identity: value.get_public_identity_key_file(),
            private_encryption: value.get_private_encryption_key_file(),
            public_encryption: value.get_public_encryption_key_file(),
            ack_key: value.get_ack_key_file(),
            gateway_shared_key: value.get_gateway_shared_key_file(),
            gateway_endpoint_config: Default::default(),
            credential_database_path: value.get_database_path(),
            reply_surb_database_path: value.get_reply_surb_database_path(),
        }
    }
}
