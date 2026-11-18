// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};
use nym_client_core::client::base_client::storage::OnDiskGatewaysDetails;
use nym_client_core::client::base_client::{non_wasm_helpers, storage};
use nym_client_core::client::key_manager::persistence::{OnDiskKeys, Passphrase};
use nym_client_core::client::replies::reply_storage::fs_backend;
use nym_client_core::config;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_client_core::config::disk_persistence::{
    ClientKeysPaths, DEFAULT_ACK_KEY_FILENAME, DEFAULT_CREDENTIALS_DB_FILENAME,
    DEFAULT_CREDENTIAL_REQUESTS_DB_FILENAME, DEFAULT_GATEWAYS_DETAILS_DB_FILENAME,
    DEFAULT_PRIVATE_ENCRYPTION_KEY_FILENAME, DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME,
    DEFAULT_PUBLIC_ENCRYPTION_KEY_FILENAME, DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME,
    DEFAULT_REPLY_SURB_DB_FILENAME,
};
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

    /// The database containing credential requests, for earlier failed attempts
    pub credential_requests_database_path: PathBuf,

    /// The database storing reply surbs in-between sessions
    pub reply_surb_database_path: PathBuf,

    /// Details of the used gateways
    pub gateway_registrations: PathBuf,

    /// Passphrase protecting the private keys; plaintext keys get encrypted on first load with it.
    key_passphrase: Option<Passphrase>,

    /// Raw SQLCipher key encrypting the credential, reply SURB and gateway registration databases;
    /// plaintext databases get encrypted on first open with it.
    database_key: Option<nym_sqlcipher::DatabaseKey>,
}

impl StoragePaths {
    /// Create a set of storage paths from a given directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if it is passed a path to an existing file instead of a
    /// directory.
    #[allow(clippy::result_large_err)]
    pub fn new_from_dir<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        if dir.is_file() {
            return Err(Error::ExpectedDirectory(dir.to_owned()));
        }

        Ok(Self {
            private_identity: dir.join(DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME),
            public_identity: dir.join(DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME),
            private_encryption: dir.join(DEFAULT_PRIVATE_ENCRYPTION_KEY_FILENAME),
            public_encryption: dir.join(DEFAULT_PUBLIC_ENCRYPTION_KEY_FILENAME),
            ack_key: dir.join(DEFAULT_ACK_KEY_FILENAME),
            credential_database_path: dir.join(DEFAULT_CREDENTIALS_DB_FILENAME),
            credential_requests_database_path: dir.join(DEFAULT_CREDENTIAL_REQUESTS_DB_FILENAME),
            reply_surb_database_path: dir.join(DEFAULT_REPLY_SURB_DB_FILENAME),
            gateway_registrations: dir.join(DEFAULT_GATEWAYS_DETAILS_DB_FILENAME),
            key_passphrase: None,
            database_key: None,
        })
    }

    /// Protects the private keys with a passphrase; keys still in plaintext are encrypted the first
    /// time they are loaded. Validating the passphrase (for instance rejecting an empty one) is the
    /// caller's job.
    #[must_use]
    pub fn with_key_passphrase(mut self, key_passphrase: Option<Passphrase>) -> Self {
        self.key_passphrase = key_passphrase;
        self
    }

    /// Encrypts the credential, reply SURB and gateway registration databases with SQLCipher,
    /// using the raw 256-bit `key` as is, without passphrase key derivation. Databases still in
    /// plaintext are encrypted the first time they are opened. The same key must be provided on
    /// every run.
    ///
    /// SQLCipher has to be linked in place of SQLite, e.g. with the `sqlcipher` feature.
    /// Otherwise opening the databases fails, without creating or changing them.
    #[must_use]
    pub fn with_database_key(mut self, key: Option<nym_sqlcipher::DatabaseKey>) -> Self {
        self.database_key = key;
        self
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

    /// Instantiates default ecash credential storage, encrypted if a database key is set.
    pub async fn persistent_credential_storage(
        &self,
    ) -> Result<PersistentCredentialStorage, Error> {
        PersistentCredentialStorage::new(&self.credential_database_path, self.database_key())
            .await
            .map_err(|source| Error::CredentialStorageError {
                source: Box::new(source),
            })
    }

    /// Instantiates default reply surb storage backend with default configuration.
    pub async fn default_persistent_fs_reply_backend(&self) -> Result<fs_backend::Backend, Error> {
        self.persistent_fs_reply_backend(&Default::default()).await
    }

    /// Instantiates default reply surb storage backend with the provided metadata config,
    /// encrypted if a database key is set.
    pub async fn persistent_fs_reply_backend(
        &self,
        surb_config: &config::ReplySurbs,
    ) -> Result<fs_backend::Backend, Error> {
        Ok(non_wasm_helpers::setup_fs_reply_surb_backend(
            &self.reply_surb_database_path,
            self.database_key(),
            surb_config,
        )
        .await?)
    }

    fn database_key(&self) -> Option<&nym_sqlcipher::DatabaseKey> {
        self.database_key.as_ref()
    }

    /// Instantiates default persistent key storage.
    pub fn on_disk_key_storage_spec(&self) -> OnDiskKeys {
        OnDiskKeys::with_passphrase(self.client_keys_paths(), self.key_passphrase.clone())
    }

    /// Instantiates gateway registrations storage, encrypted if a database key is set.
    pub async fn on_disk_gateway_details_storage(&self) -> Result<OnDiskGatewaysDetails, Error> {
        Ok(non_wasm_helpers::setup_fs_gateways_storage(
            &self.gateway_registrations,
            self.database_key(),
        )
        .await?)
    }

    /// Gives the client a new identity: new random root keys, no gateway registrations and no
    /// reply SURBs or reply keys. It contacts no gateway and leaves the credential store unopened,
    /// so ticketbooks are kept. No client may be using these paths while it runs. It fails before
    /// changing anything when the key passphrase does not open the existing keys.
    pub async fn reset_client_identity(&self) -> Result<()> {
        Ok(non_wasm_helpers::reset_client_identity(
            &self.common_client_paths(),
            self.key_passphrase.clone(),
            self.database_key(),
        )
        .await?)
    }

    pub fn credential_database_paths(&self) -> Vec<PathBuf> {
        non_wasm_helpers::sqlite_database_paths(&self.credential_database_path)
    }

    pub fn reply_surb_database_paths(&self) -> Vec<PathBuf> {
        non_wasm_helpers::sqlite_database_paths(&self.reply_surb_database_path)
    }

    pub fn gateway_registrations_paths(&self) -> Vec<PathBuf> {
        non_wasm_helpers::sqlite_database_paths(&self.gateway_registrations)
    }

    fn common_client_paths(&self) -> CommonClientPaths {
        CommonClientPaths {
            keys: self.client_keys_paths(),
            gateway_registrations: self.gateway_registrations.clone(),
            credential_requests_database: self.credential_requests_database_path.clone(),
            credentials_database: self.credential_database_path.clone(),
            reply_surb_database: self.reply_surb_database_path.clone(),
        }
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
        value.common_client_paths()
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
            credential_requests_database_path: value.credential_requests_database,
            reply_surb_database_path: value.reply_surb_database,
            gateway_registrations: value.gateway_registrations,
            key_passphrase: None,
            database_key: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_client_core::client::base_client::storage::gateways_storage::{
        GatewayDetails, GatewaysDetailsStore,
    };
    use nym_client_core::client::key_manager::persistence::KeyStore;
    use nym_client_core::client::key_manager::ClientKeys;
    use nym_client_core::client::replies::reply_storage::ReplyStorageBackend;
    use nym_client_core::error::ClientCoreError;
    use nym_test_utils::helpers::{deterministic_rng, dummy_ed25519_keypair};

    #[tokio::test]
    async fn key_passphrase_reaches_the_key_store() {
        let dir = tempfile::tempdir().unwrap();
        let paths = StoragePaths::new_from_dir(dir.path())
            .unwrap()
            .with_key_passphrase(Some(Passphrase::new("hunter2")));

        paths
            .on_disk_key_storage_spec()
            .store_keys(&ClientKeys::generate_new(&mut deterministic_rng()))
            .await
            .unwrap();

        assert!(nym_pemstore::is_encrypted(&paths.private_identity).unwrap());
        assert!(nym_pemstore::is_encrypted(&paths.private_encryption).unwrap());
        assert!(nym_pemstore::is_encrypted(&paths.ack_key).unwrap());
        assert!(!nym_pemstore::is_encrypted(&paths.public_identity).unwrap());
    }

    #[tokio::test]
    async fn without_a_passphrase_keys_stay_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let paths = StoragePaths::new_from_dir(dir.path()).unwrap();

        paths
            .on_disk_key_storage_spec()
            .store_keys(&ClientKeys::generate_new(&mut deterministic_rng()))
            .await
            .unwrap();

        assert!(!nym_pemstore::is_encrypted(&paths.private_identity).unwrap());
    }

    #[tokio::test]
    async fn reset_client_identity_replaces_the_root_and_clears_registrations_and_replies() {
        let dir = tempfile::tempdir().unwrap();
        let paths = StoragePaths::new_from_dir(dir.path()).unwrap();

        let key_store = paths.on_disk_key_storage_spec();
        let old_keys = ClientKeys::generate_new(&mut deterministic_rng());
        key_store.store_keys(&old_keys).await.unwrap();

        let gateway_id = *dummy_ed25519_keypair(1).public_key();
        let details_store = paths.on_disk_gateway_details_storage().await.unwrap();
        details_store
            .store_gateway_details(&GatewayDetails::new_custom(gateway_id, None).into())
            .await
            .unwrap();
        details_store
            .set_active_gateway(&gateway_id.to_base58_string())
            .await
            .unwrap();

        let reply_store = paths.default_persistent_fs_reply_backend().await.unwrap();
        reply_store.stop_storage_session().await.unwrap();
        assert!(paths.reply_surb_database_path.exists());

        paths.reset_client_identity().await.unwrap();

        let new_keys = key_store.load_keys().await.unwrap();
        assert_ne!(
            new_keys.identity_keypair().public_key(),
            old_keys.identity_keypair().public_key()
        );
        assert!(details_store
            .all_gateways_identities()
            .await
            .unwrap()
            .is_empty());
        assert!(details_store
            .active_gateway()
            .await
            .unwrap()
            .registration
            .is_none());
        assert!(paths
            .reply_surb_database_paths()
            .iter()
            .all(|path| !path.exists()));
    }

    #[tokio::test]
    async fn reset_client_identity_changes_nothing_without_the_key_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let paths = StoragePaths::new_from_dir(dir.path())
            .unwrap()
            .with_key_passphrase(Some(Passphrase::new("hunter2")));

        let old_keys = ClientKeys::generate_new(&mut deterministic_rng());
        paths
            .on_disk_key_storage_spec()
            .store_keys(&old_keys)
            .await
            .unwrap();

        let gateway_id = *dummy_ed25519_keypair(1).public_key();
        let details_store = paths.on_disk_gateway_details_storage().await.unwrap();
        details_store
            .store_gateway_details(&GatewayDetails::new_custom(gateway_id, None).into())
            .await
            .unwrap();

        for key_passphrase in [None, Some(Passphrase::new("hunter3"))] {
            let reset = paths
                .clone()
                .with_key_passphrase(key_passphrase)
                .reset_client_identity()
                .await;
            assert!(matches!(
                reset,
                Err(Error::ClientCoreError(
                    ClientCoreError::KeyStoreError { .. }
                ))
            ));
        }

        let keys = paths.on_disk_key_storage_spec().load_keys().await.unwrap();
        assert_eq!(
            keys.identity_keypair().public_key(),
            old_keys.identity_keypair().public_key()
        );
        assert_eq!(
            details_store.all_gateways_identities().await.unwrap().len(),
            1
        );
    }
}
