// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};
use nym_client_core::client::base_client::storage::OnDiskGatewaysDetails;
use nym_client_core::client::base_client::{non_wasm_helpers, storage};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::client::replies::reply_storage::fs_backend;
use nym_client_core::config;
use nym_client_core::config::disk_persistence::ClientKeysPaths;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_credential_storage::persistent_storage::PersistentStorage as PersistentCredentialStorage;
use std::path::{Path, PathBuf};

/// Set of storage paths that the client will use if it is setup to persist keys, credentials, and
/// reply-SURBs.
#[derive(Clone, Debug)]
pub struct StoragePaths {
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

    /// The database containing credentials
    pub credential_database_path: PathBuf,

    /// The database storing reply surbs in-between sessions
    pub reply_surb_database_path: PathBuf,

    /// Details of the used gateways
    pub gateway_registrations: PathBuf,
}

impl StoragePaths {
    /// Create a set of storage paths from a given directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if it is passed a path to an existing file instead of a
    /// directory.
    pub fn new_from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        if dir.is_file() {
            return Err(Error::ExpectedDirectory(dir.to_owned()));
        }

        Ok(Self {
            private_identity: dir.join("private_identity.pem"),
            public_identity: dir.join("public_identity.pem"),
            private_encryption: dir.join("private_encryption.pem"),
            public_encryption: dir.join("public_encryption.pem"),
            ack_key: dir.join("ack_key.pem"),
            credential_database_path: dir.join("db.sqlite"),
            reply_surb_database_path: dir.join("persistent_reply_store.sqlite"),
            gateway_registrations: dir.join("gateways_registrations.sqlite"),
        })
    }

    /// Instantiates default full client storage backend with default configuration.
    pub async fn initialise_default_persistent_storage(
        &self,
    ) -> Result<storage::OnDiskPersistent, Error> {
        Ok(storage::OnDiskPersistent::new(
            self.on_disk_key_storage_spec(),
            self.default_persistent_fs_reply_backend().await?,
            self.persistent_credential_storage().await?,
            self.on_disk_gateway_details_storage().await?,
        ))
    }

    /// Instantiates default full client storage backend with the provided configuration.
    pub async fn initialise_persistent_storage(
        &self,
        config: &config::DebugConfig,
    ) -> Result<storage::OnDiskPersistent, Error> {
        Ok(storage::OnDiskPersistent::new(
            self.on_disk_key_storage_spec(),
            self.persistent_fs_reply_backend(&config.reply_surbs)
                .await?,
            self.persistent_credential_storage().await?,
            self.on_disk_gateway_details_storage().await?,
        ))
    }

    /// Instantiates default coconut credential storage.
    pub async fn persistent_credential_storage(
        &self,
    ) -> Result<PersistentCredentialStorage, Error> {
        PersistentCredentialStorage::init(&self.credential_database_path)
            .await
            .map_err(|source| Error::CredentialStorageError {
                source: Box::new(source),
            })
    }

    /// Instantiates default reply surb storage backend with default configuration.
    pub async fn default_persistent_fs_reply_backend(&self) -> Result<fs_backend::Backend, Error> {
        self.persistent_fs_reply_backend(&Default::default()).await
    }

    /// Instantiates default reply surb storage backend with the provided metadata config.
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

    /// Instantiates default persistent key storage.
    pub fn on_disk_key_storage_spec(&self) -> OnDiskKeys {
        OnDiskKeys::new(self.client_keys_paths())
    }

    pub async fn on_disk_gateway_details_storage(&self) -> Result<OnDiskGatewaysDetails, Error> {
        Ok(non_wasm_helpers::setup_fs_gateways_storage(&self.gateway_registrations).await?)
    }

    fn client_keys_paths(&self) -> ClientKeysPaths {
        ClientKeysPaths {
            private_identity_key_file: self.private_identity.clone(),
            public_identity_key_file: self.public_identity.clone(),
            private_encryption_key_file: self.private_encryption.clone(),
            public_encryption_key_file: self.public_encryption.clone(),
            ack_key_file: self.ack_key.clone(),
        }
    }
}

impl From<StoragePaths> for CommonClientPaths {
    fn from(value: StoragePaths) -> Self {
        CommonClientPaths {
            keys: ClientKeysPaths {
                private_identity_key_file: value.private_identity,
                public_identity_key_file: value.public_identity,
                private_encryption_key_file: value.private_encryption,
                public_encryption_key_file: value.public_encryption,
                ack_key_file: value.ack_key,
            },
            gateway_registrations: value.gateway_registrations,
            credentials_database: value.credential_database_path,
            reply_surb_database: value.reply_surb_database_path,
        }
    }
}

impl From<CommonClientPaths> for StoragePaths {
    fn from(value: CommonClientPaths) -> Self {
        StoragePaths {
            private_identity: value.keys.private_identity_key_file,
            public_identity: value.keys.public_identity_key_file,
            private_encryption: value.keys.private_encryption_key_file,
            public_encryption: value.keys.public_encryption_key_file,
            ack_key: value.keys.ack_key_file,
            credential_database_path: value.credentials_database,
            reply_surb_database_path: value.reply_surb_database,
            gateway_registrations: value.gateway_registrations,
        }
    }
}
