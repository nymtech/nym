// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::ClientKeys;
use async_trait::async_trait;
use rand::CryptoRng;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use crate::config::disk_persistence::ClientKeysPaths;
#[cfg(not(target_arch = "wasm32"))]
use nym_crypto::asymmetric::{ed25519, x25519};
#[cfg(not(target_arch = "wasm32"))]
use nym_pemstore::KeyPairPath;
#[cfg(not(target_arch = "wasm32"))]
pub use nym_pemstore::Passphrase;
#[cfg(not(target_arch = "wasm32"))]
use nym_pemstore::PassphraseError;
#[cfg(not(target_arch = "wasm32"))]
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
#[cfg(not(target_arch = "wasm32"))]
use nym_sphinx::acknowledgements::AckKey;

/// Error of a [`KeyStore`]; every store must say whether a failed load means no keys exist yet.
pub trait KeyStoreError: Error + Send + Sync + 'static {
    /// Whether no keys are stored yet, so a fresh set may be generated; `false` protects existing keys that could not be read.
    fn keys_missing(&self) -> bool;
}

// we have to define it as an async trait since wasm storage is async
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait KeyStore {
    type StorageError: KeyStoreError;

    async fn load_keys(&self) -> Result<ClientKeys, Self::StorageError>;

    async fn store_keys(&self, keys: &ClientKeys) -> Result<(), Self::StorageError>;
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

    #[error(
        "the {keys} private key at {path} is encrypted: a key passphrase is required to use these keys"
    )]
    PassphraseRequired { keys: String, path: String },

    #[error(
        "the {keys} private key at {path} could not be decrypted with the given key passphrase (wrong passphrase or corrupted key file)"
    )]
    WrongPassphrase { keys: String, path: String },
}

/// The explicit passphrase failure behind a pemstore load error, if that is what it was.
#[cfg(not(target_arch = "wasm32"))]
fn passphrase_failure(keys: &str, err: &std::io::Error) -> Option<OnDiskKeysError> {
    let failure = match nym_pemstore::passphrase_error(err)? {
        PassphraseError::Required { path } => OnDiskKeysError::PassphraseRequired {
            keys: keys.to_string(),
            path: path.display().to_string(),
        },
        PassphraseError::Rejected { path, .. } => OnDiskKeysError::WrongPassphrase {
            keys: keys.to_string(),
            path: path.display().to_string(),
        },
    };
    Some(failure)
}

#[derive(Clone)]
#[cfg(not(target_arch = "wasm32"))]
pub struct OnDiskKeys {
    paths: ClientKeysPaths,
    key_passphrase: Option<Passphrase>,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<ClientKeysPaths> for OnDiskKeys {
    fn from(paths: ClientKeysPaths) -> Self {
        OnDiskKeys::new(paths)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl OnDiskKeys {
    pub fn new(paths: ClientKeysPaths) -> Self {
        OnDiskKeys::with_passphrase(paths, None)
    }

    /// With a passphrase, private keys are stored encrypted and plaintext ones are re-encrypted on load.
    pub fn with_passphrase(paths: ClientKeysPaths, key_passphrase: Option<Passphrase>) -> Self {
        OnDiskKeys {
            paths,
            key_passphrase,
        }
    }

    #[doc(hidden)]
    pub fn load_encryption_keypair(&self) -> Result<x25519::KeyPair, OnDiskKeysError> {
        let encryption_paths = self.paths.encryption_key_pair_path();
        self.load_keypair(encryption_paths, "encryption")
    }

    #[doc(hidden)]
    pub fn load_identity_keypair(&self) -> Result<ed25519::KeyPair, OnDiskKeysError> {
        let identity_paths = self.paths.identity_key_pair_path();
        self.load_keypair(identity_paths, "identity")
    }

    fn load_key<T: PemStorableKey>(
        &self,
        path: &std::path::Path,
        name: impl Into<String>,
    ) -> Result<T, OnDiskKeysError> {
        nym_pemstore::load_key_with(path, self.key_passphrase.as_ref()).map_err(|err| {
            let key = name.into();
            passphrase_failure(&key, &err).unwrap_or_else(|| OnDiskKeysError::KeyLoadFailure {
                path: path.to_str().map(|s| s.to_owned()).unwrap_or_default(),
                key,
                err,
            })
        })
    }

    fn load_keypair<T: PemStorableKeyPair>(
        &self,
        paths: KeyPairPath,
        name: impl Into<String>,
    ) -> Result<T, OnDiskKeysError> {
        nym_pemstore::load_keypair_with(&paths, self.key_passphrase.as_ref()).map_err(|err| {
            let keys = name.into();
            passphrase_failure(&keys, &err).unwrap_or_else(|| OnDiskKeysError::KeyPairLoadFailure {
                keys,
                paths,
                err,
            })
        })
    }

    fn store_key<T: PemStorableKey>(
        &self,
        key: &T,
        path: &std::path::Path,
        name: impl Into<String>,
    ) -> Result<(), OnDiskKeysError> {
        nym_pemstore::store_key_with(key, path, self.key_passphrase.as_ref()).map_err(|err| {
            OnDiskKeysError::KeyStoreFailure {
                key: name.into(),
                path: path.to_str().map(|s| s.to_owned()).unwrap_or_default(),
                err,
            }
        })
    }

    fn store_keypair<T: PemStorableKeyPair>(
        &self,
        keys: &T,
        paths: KeyPairPath,
        name: impl Into<String>,
    ) -> Result<(), OnDiskKeysError> {
        nym_pemstore::store_keypair_with(keys, &paths, self.key_passphrase.as_ref()).map_err(
            |err| OnDiskKeysError::KeyPairStoreFailure {
                keys: name.into(),
                paths,
                err,
            },
        )
    }

    fn load_keys(&self) -> Result<ClientKeys, OnDiskKeysError> {
        let identity_keypair = self.load_identity_keypair()?;
        let encryption_keypair = self.load_encryption_keypair()?;
        let ack_key: AckKey = self.load_key(self.paths.ack_key(), "ack key")?;
        let keys = ClientKeys::from_keys(identity_keypair, encryption_keypair, ack_key);

        if self.key_passphrase.is_some() && !self.private_keys_encrypted()? {
            tracing::info!(
                "encrypting the client's plaintext private keys with the provided passphrase"
            );
            self.store_keys(&keys)?;
        }

        Ok(keys)
    }

    fn private_keys_encrypted(&self) -> Result<bool, OnDiskKeysError> {
        let identity_encrypted = self.is_encrypted(
            "ed25519 identity",
            &self.paths.identity_key_pair_path().private_key_path,
        )?;

        let encryption_encrypted = self.is_encrypted(
            "x25519 encryption",
            &self.paths.encryption_key_pair_path().private_key_path,
        )?;

        let ack_encrypted = self.is_encrypted("ack key", self.paths.ack_key())?;

        Ok(identity_encrypted && encryption_encrypted && ack_encrypted)
    }

    fn is_encrypted(&self, name: &str, path: &std::path::Path) -> Result<bool, OnDiskKeysError> {
        nym_pemstore::is_encrypted(path).map_err(|err| OnDiskKeysError::KeyLoadFailure {
            key: name.to_string(),
            path: path.to_str().map(|s| s.to_owned()).unwrap_or_default(),
            err,
        })
    }

    fn store_keys(&self, keys: &ClientKeys) -> Result<(), OnDiskKeysError> {
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

    async fn load_keys(&self) -> Result<ClientKeys, Self::StorageError> {
        self.load_keys()
    }

    async fn store_keys(&self, keys: &ClientKeys) -> Result<(), Self::StorageError> {
        self.store_keys(keys)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl KeyStoreError for OnDiskKeysError {
    fn keys_missing(&self) -> bool {
        match self {
            OnDiskKeysError::KeyLoadFailure { err, .. }
            | OnDiskKeysError::KeyPairLoadFailure { err, .. } => {
                err.kind() == std::io::ErrorKind::NotFound
            }
            OnDiskKeysError::PassphraseRequired { .. }
            | OnDiskKeysError::WrongPassphrase { .. }
            | OnDiskKeysError::KeyStoreFailure { .. }
            | OnDiskKeysError::KeyPairStoreFailure { .. } => false,
        }
    }
}

#[derive(Clone)]
pub struct InMemEphemeralKeys {
    keys: Arc<Mutex<ClientKeys>>,
}

impl InMemEphemeralKeys {
    pub fn new<R>(rng: &mut R) -> Self
    where
        R: CryptoRng,
    {
        InMemEphemeralKeys {
            keys: Arc::new(Mutex::new(ClientKeys::generate_new(rng))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("old ephemeral keys can't be loaded from storage")]
pub struct EphemeralKeysError;

// never produced in practice: loading from the in-memory store cannot fail
impl KeyStoreError for EphemeralKeysError {
    fn keys_missing(&self) -> bool {
        true
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl KeyStore for InMemEphemeralKeys {
    type StorageError = EphemeralKeysError;

    async fn load_keys(&self) -> Result<ClientKeys, Self::StorageError> {
        Ok(self.keys.lock().await.clone())
    }

    async fn store_keys(&self, keys: &ClientKeys) -> Result<(), Self::StorageError> {
        *self.keys.lock().await = keys.clone();
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use nym_test_utils::helpers::deterministic_rng;
    use std::path::{Path, PathBuf};

    fn paths(dir: &Path) -> ClientKeysPaths {
        ClientKeysPaths {
            private_identity_key_file: dir.join("private_identity.pem"),
            public_identity_key_file: dir.join("public_identity.pem"),
            private_encryption_key_file: dir.join("private_encryption.pem"),
            public_encryption_key_file: dir.join("public_encryption.pem"),
            ack_key_file: dir.join("ack_key.pem"),
        }
    }

    fn passphrase() -> Option<Passphrase> {
        Some(Passphrase::new("hunter2"))
    }

    fn fresh_keys() -> ClientKeys {
        ClientKeys::generate_new(&mut deterministic_rng())
    }

    // the key types' inherent `to_bytes` return fixed arrays (the ack key a Vec); `to_vec` unifies them
    fn secret_bytes(keys: &ClientKeys) -> [Vec<u8>; 3] {
        [
            keys.identity_keypair().private_key().to_bytes().to_vec(),
            keys.encryption_keypair().private_key().to_bytes().to_vec(),
            keys.ack_key().to_bytes().to_vec(),
        ]
    }

    // identity, encryption, ack
    #[allow(clippy::unwrap_used)]
    fn private_keys_encrypted(paths: &ClientKeysPaths) -> [bool; 3] {
        [
            nym_pemstore::is_encrypted(&paths.private_identity_key_file).unwrap(),
            nym_pemstore::is_encrypted(&paths.private_encryption_key_file).unwrap(),
            nym_pemstore::is_encrypted(&paths.ack_key_file).unwrap(),
        ]
    }

    #[allow(clippy::unwrap_used)]
    fn public_keys_plaintext(paths: &ClientKeysPaths) -> bool {
        !nym_pemstore::is_encrypted(&paths.public_identity_key_file).unwrap()
            && !nym_pemstore::is_encrypted(&paths.public_encryption_key_file).unwrap()
    }

    #[test]
    fn store_with_passphrase_encrypts_private_keys_from_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        let keys = fresh_keys();

        OnDiskKeys::with_passphrase(paths.clone(), passphrase())
            .store_keys(&keys)
            .unwrap();

        assert_eq!(private_keys_encrypted(&paths), [true, true, true]);
        assert!(public_keys_plaintext(&paths));
    }

    #[test]
    fn load_without_passphrase_keeps_plaintext_keys_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        let keys = fresh_keys();
        OnDiskKeys::new(paths.clone()).store_keys(&keys).unwrap();

        let loaded = OnDiskKeys::new(paths.clone()).load_keys().unwrap();

        assert_eq!(secret_bytes(&loaded), secret_bytes(&keys));
        assert_eq!(private_keys_encrypted(&paths), [false, false, false]);
    }

    #[test]
    fn load_with_passphrase_re_saves_plaintext_keys_encrypted() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        let keys = fresh_keys();
        OnDiskKeys::new(paths.clone()).store_keys(&keys).unwrap();

        let loaded = OnDiskKeys::with_passphrase(paths.clone(), passphrase())
            .load_keys()
            .unwrap();

        assert_eq!(secret_bytes(&loaded), secret_bytes(&keys));
        assert_eq!(private_keys_encrypted(&paths), [true, true, true]);
        assert!(public_keys_plaintext(&paths));
    }

    #[test]
    fn load_with_passphrase_completes_a_partial_migration() {
        // identity encrypted, the rest plaintext: what a crash between two re-save writes leaves
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        let keys = fresh_keys();
        OnDiskKeys::new(paths.clone()).store_keys(&keys).unwrap();
        nym_pemstore::store_keypair_with(
            keys.identity_keypair().as_ref(),
            &paths.identity_key_pair_path(),
            passphrase().as_ref(),
        )
        .unwrap();
        assert_eq!(private_keys_encrypted(&paths), [true, false, false]);

        let loaded = OnDiskKeys::with_passphrase(paths.clone(), passphrase())
            .load_keys()
            .unwrap();

        assert_eq!(secret_bytes(&loaded), secret_bytes(&keys));
        assert_eq!(private_keys_encrypted(&paths), [true, true, true]);
    }

    #[test]
    fn load_with_passphrase_leaves_encrypted_keys_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        let store = OnDiskKeys::with_passphrase(paths.clone(), passphrase());
        store.store_keys(&fresh_keys()).unwrap();
        let before = std::fs::read(&paths.private_identity_key_file).unwrap();

        store.load_keys().unwrap();

        // a re-save picks a fresh salt and nonce, so identical bytes prove nothing was rewritten
        assert_eq!(
            std::fs::read(&paths.private_identity_key_file).unwrap(),
            before
        );
    }

    #[test]
    fn load_without_passphrase_rejects_encrypted_keys() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        OnDiskKeys::with_passphrase(paths.clone(), passphrase())
            .store_keys(&fresh_keys())
            .unwrap();

        let Err(err) = OnDiskKeys::new(paths.clone()).load_keys() else {
            panic!("loading encrypted keys without a passphrase must fail")
        };

        assert!(
            matches!(err, OnDiskKeysError::PassphraseRequired { .. }),
            "{err}"
        );
    }

    #[test]
    fn load_with_wrong_passphrase_fails() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        OnDiskKeys::with_passphrase(paths.clone(), passphrase())
            .store_keys(&fresh_keys())
            .unwrap();

        let Err(err) = OnDiskKeys::with_passphrase(paths.clone(), Some(Passphrase::new("hunter3")))
            .load_keys()
        else {
            panic!("loading with a wrong passphrase must fail")
        };

        assert!(
            matches!(err, OnDiskKeysError::WrongPassphrase { .. }),
            "{err}"
        );
    }

    #[test]
    fn re_save_failure_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());
        let keys = fresh_keys();
        OnDiskKeys::new(paths.clone()).store_keys(&keys).unwrap();
        // pemstore writes through a sibling `.tmp` file; a directory in its place makes that write fail
        let blocked = PathBuf::from(format!("{}.tmp", paths.private_identity_key_file.display()));
        std::fs::create_dir(&blocked).unwrap();

        let Err(err) = OnDiskKeys::with_passphrase(paths.clone(), passphrase()).load_keys() else {
            panic!("a failed re-save must surface as an error")
        };

        assert!(
            matches!(err, OnDiskKeysError::KeyPairStoreFailure { .. }),
            "{err}"
        );
        // a failed re-save must never be followed by regeneration either
        assert!(!err.keys_missing(), "{err}");
        assert_eq!(private_keys_encrypted(&paths), [false, false, false]);
    }

    // the base client generates a fresh set when keys are missing; it must never do so when
    // existing keys merely could not be read
    #[test]
    fn only_absent_files_count_as_missing_keys() {
        let dir = tempfile::tempdir().unwrap();
        let paths = paths(dir.path());

        let Err(absent) = OnDiskKeys::new(paths.clone()).load_keys() else {
            panic!("nothing has been stored yet")
        };
        assert!(absent.keys_missing(), "{absent}");

        OnDiskKeys::with_passphrase(paths.clone(), passphrase())
            .store_keys(&fresh_keys())
            .unwrap();
        let Err(no_passphrase) = OnDiskKeys::new(paths.clone()).load_keys() else {
            panic!("loading encrypted keys without a passphrase must fail")
        };
        assert!(!no_passphrase.keys_missing(), "{no_passphrase}");

        let Err(wrong_passphrase) =
            OnDiskKeys::with_passphrase(paths.clone(), Some(Passphrase::new("hunter3")))
                .load_keys()
        else {
            panic!("loading with a wrong passphrase must fail")
        };
        assert!(!wrong_passphrase.keys_missing(), "{wrong_passphrase}");
    }
}
