// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};
use nym_client_core::client::base_client::storage::helpers::{
    get_all_registered_identities, remove_gateway_details,
};
use nym_client_core::client::base_client::storage::OnDiskGatewaysDetails;
use nym_client_core::client::base_client::{non_wasm_helpers, storage};
use nym_client_core::client::key_manager::persistence::{KeySecret, OnDiskKeys};
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
use nym_client_core::init::generate_new_client_keys;
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

    /// Secret (passphrase or raw key) protecting the private keys; plaintext keys get encrypted on
    /// first load with it.
    key_secret: Option<KeySecret>,

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
            key_secret: None,
            database_key: None,
        })
    }

    /// Protects the private keys with a secret: a passphrase, or a uniformly random 256-bit raw
    /// key that needs no stretching. Keys still in plaintext are encrypted the first time they
    /// are loaded. Validating the secret (for instance rejecting an empty passphrase) is the
    /// caller's job.
    #[must_use]
    pub fn with_key_secret(mut self, key_secret: Option<KeySecret>) -> Self {
        self.key_secret = key_secret;
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
        OnDiskKeys::with_secret(self.client_keys_paths(), self.key_secret.clone())
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
    /// so ticketbooks are kept. No client may be using these paths while it runs.
    pub async fn reset_client_identity(&self) -> Result<()> {
        // the keys go last, so a reset cut short leaves registrations under the keys they were
        // made with, or none, and running it again finishes it
        let details_store = self.on_disk_gateway_details_storage().await?;
        for gateway_id in get_all_registered_identities(&details_store).await? {
            remove_gateway_details(&details_store, &gateway_id.to_base58_string()).await?;
        }

        // loaded directly: the usual setup would create a missing store with default thresholds,
        // which the client would then keep
        if self.reply_surb_database_path.exists() {
            match fs_backend::Backend::try_load(&self.reply_surb_database_path, self.database_key())
                .await
            {
                Ok(reply_store) => {
                    let deleted = reply_store.delete_all_reply_data().await;
                    reply_store.shutdown().await;
                    deleted.map_err(|source| Error::ReplyStorageError {
                        source: Box::new(source),
                    })?;
                }
                // a flush cut short or never made: the client's next start would archive this
                // store and begin a fresh one, so deleting it loses nothing a start would keep.
                // Any other failure, a wrong key included, may hide data the client can still read
                Err(fs_backend::StorageError::IncompleteDataFlush) => {
                    for path in self.reply_surb_database_paths() {
                        if path.exists() {
                            std::fs::remove_file(path)?;
                        }
                    }
                }
                Err(source) => {
                    return Err(Error::ReplyStorageError {
                        source: Box::new(source),
                    })
                }
            }
        }

        let mut rng = nym_crypto::rng::os_rng();
        generate_new_client_keys(&mut rng, &self.on_disk_key_storage_spec()).await?;
        Ok(())
    }

    pub fn credential_database_paths(&self) -> Vec<PathBuf> {
        Self::with_sqlite_journal_paths(&self.credential_database_path)
    }

    pub fn reply_surb_database_paths(&self) -> Vec<PathBuf> {
        Self::with_sqlite_journal_paths(&self.reply_surb_database_path)
    }

    pub fn gateway_registrations_paths(&self) -> Vec<PathBuf> {
        Self::with_sqlite_journal_paths(&self.gateway_registrations)
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

    /// Returns a list of paths that include the sqlite database and journal files (wal, shm)
    fn with_sqlite_journal_paths<P: AsRef<Path>>(db_file: P) -> Vec<PathBuf> {
        ["-shm", "-wal"]
            .iter()
            .map(|ext_suffix| {
                let mut new_ext = db_file.as_ref().extension().unwrap_or_default().to_owned();
                new_ext.push(ext_suffix);
                db_file.as_ref().with_extension(new_ext)
            })
            .chain([db_file.as_ref().to_path_buf()])
            .collect()
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
            credential_requests_database: value.credential_requests_database_path,
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
            credential_requests_database_path: value.credential_requests_database,
            reply_surb_database_path: value.reply_surb_database,
            gateway_registrations: value.gateway_registrations,
            key_secret: None,
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
    use nym_client_core::client::key_manager::persistence::{KeyStore, Passphrase, RawKey};
    use nym_client_core::client::key_manager::ClientKeys;
    use nym_client_core::client::replies::reply_storage::{
        CombinedReplyStorage, ReplyStorageBackend,
    };
    use nym_sphinx::anonymous_replies::SurbEncryptionKey;
    use nym_test_utils::helpers::{deterministic_rng, dummy_ed25519_keypair};
    use time::OffsetDateTime;

    #[tokio::test]
    async fn key_passphrase_reaches_the_key_store() {
        let dir = tempfile::tempdir().unwrap();
        let paths = StoragePaths::new_from_dir(dir.path())
            .unwrap()
            .with_key_secret(Some(Passphrase::new("hunter2").into()));

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
    async fn raw_key_reaches_the_key_store() {
        let dir = tempfile::tempdir().unwrap();
        let secret = KeySecret::from(RawKey::from_bytes(&[7; nym_pemstore::RAW_KEY_LEN]));
        let paths = StoragePaths::new_from_dir(dir.path())
            .unwrap()
            .with_key_secret(Some(secret.clone()));
        let keys = ClientKeys::generate_new(&mut deterministic_rng());

        paths
            .on_disk_key_storage_spec()
            .store_keys(&keys)
            .await
            .unwrap();

        assert!(nym_pemstore::is_encrypted(&paths.private_identity).unwrap());
        assert!(nym_pemstore::is_encrypted(&paths.private_encryption).unwrap());
        assert!(nym_pemstore::is_encrypted(&paths.ack_key).unwrap());
        assert!(!nym_pemstore::is_encrypted(&paths.public_identity).unwrap());
        let loaded = paths.on_disk_key_storage_spec().load_keys().await.unwrap();
        assert_eq!(
            loaded.identity_keypair().private_key().to_bytes(),
            keys.identity_keypair().private_key().to_bytes()
        );
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
        let mut rng = deterministic_rng();

        let key_store = paths.on_disk_key_storage_spec();
        let old_keys = ClientKeys::generate_new(&mut rng);
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

        let replies = CombinedReplyStorage::new(10, 100);
        replies
            .key_storage_ref()
            .insert_multiple(vec![SurbEncryptionKey::new(&mut rng)]);
        let mut reply_store = paths.default_persistent_fs_reply_backend().await.unwrap();
        reply_store.flush_surb_storage(&replies).await.unwrap();
        reply_store.stop_storage_session().await.unwrap();
        assert_eq!(stored_reply_data(&paths).await, (1, 0));

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
        assert_eq!(stored_reply_data(&paths).await, (0, 0));
    }

    #[tokio::test]
    async fn reset_client_identity_deletes_a_reply_store_that_was_never_flushed() {
        let dir = tempfile::tempdir().unwrap();
        let paths = StoragePaths::new_from_dir(dir.path()).unwrap();

        let key_store = paths.on_disk_key_storage_spec();
        let old_keys = ClientKeys::generate_new(&mut deterministic_rng());
        key_store.store_keys(&old_keys).await.unwrap();

        // created and closed without a flush, as by a client that went down before its first one
        let reply_store = paths.default_persistent_fs_reply_backend().await.unwrap();
        reply_store.stop_storage_session().await.unwrap();
        assert!(matches!(
            fs_backend::Backend::try_load(&paths.reply_surb_database_path, None).await,
            Err(fs_backend::StorageError::IncompleteDataFlush)
        ));

        paths.reset_client_identity().await.unwrap();

        assert_ne!(
            key_store
                .load_keys()
                .await
                .unwrap()
                .identity_keypair()
                .public_key(),
            old_keys.identity_keypair().public_key()
        );
        assert!(paths
            .reply_surb_database_paths()
            .iter()
            .all(|path| !path.exists()));
    }

    /// The numbers of stored reply keys and of senders with stored reply SURBs.
    async fn stored_reply_data(paths: &StoragePaths) -> (usize, usize) {
        let reply_store = fs_backend::Backend::try_load(&paths.reply_surb_database_path, None)
            .await
            .unwrap();
        let stored = reply_store
            .load_surb_storage(OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        reply_store.shutdown().await;
        (
            stored.key_storage_ref().as_raw_iter().count(),
            stored.surbs_storage_ref().as_raw_iter().count(),
        )
    }
}
