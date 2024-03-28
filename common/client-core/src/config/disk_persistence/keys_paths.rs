// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME: &str = "private_identity.pem";
pub const DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME: &str = "public_identity.pem";
pub const DEFAULT_PRIVATE_ENCRYPTION_KEY_FILENAME: &str = "private_encryption.pem";
pub const DEFAULT_PUBLIC_ENCRYPTION_KEY_FILENAME: &str = "public_encryption.pem";
pub const DEFAULT_ACK_KEY_FILENAME: &str = "ack_key.pem";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct ClientKeysPaths {
    /// Path to file containing private identity key.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    pub public_identity_key_file: PathBuf,

    /// Path to file containing private encryption key.
    pub private_encryption_key_file: PathBuf,

    /// Path to file containing public encryption key.
    pub public_encryption_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,
}

impl ClientKeysPaths {
    pub fn new_base<P: AsRef<Path>>(base_data_directory: P) -> Self {
        let base_dir = base_data_directory.as_ref();

        ClientKeysPaths {
            private_identity_key_file: base_dir.join(DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME),
            public_identity_key_file: base_dir.join(DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME),
            private_encryption_key_file: base_dir.join(DEFAULT_PRIVATE_ENCRYPTION_KEY_FILENAME),
            public_encryption_key_file: base_dir.join(DEFAULT_PUBLIC_ENCRYPTION_KEY_FILENAME),
            ack_key_file: base_dir.join(DEFAULT_ACK_KEY_FILENAME),
        }
    }

    pub fn identity_key_pair_path(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            self.private_identity_key().to_path_buf(),
            self.public_identity_key().to_path_buf(),
        )
    }

    pub fn encryption_key_pair_path(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            self.private_encryption_key().to_path_buf(),
            self.public_encryption_key().to_path_buf(),
        )
    }

    pub fn any_file_exists(&self) -> bool {
        matches!(self.public_identity_key_file.try_exists(), Ok(true))
            || matches!(self.private_identity_key_file.try_exists(), Ok(true))
            || matches!(self.public_encryption_key_file.try_exists(), Ok(true))
            || matches!(self.private_encryption_key_file.try_exists(), Ok(true))
            || matches!(self.ack_key_file.try_exists(), Ok(true))
    }

    pub fn any_file_exists_and_return(&self) -> Option<PathBuf> {
        file_exists(&self.public_identity_key_file)
            .or_else(|| file_exists(&self.private_identity_key_file))
            .or_else(|| file_exists(&self.public_encryption_key_file))
            .or_else(|| file_exists(&self.private_encryption_key_file))
            .or_else(|| file_exists(&self.ack_key_file))
    }

    pub fn private_identity_key(&self) -> &Path {
        &self.private_identity_key_file
    }

    pub fn public_identity_key(&self) -> &Path {
        &self.public_identity_key_file
    }

    pub fn private_encryption_key(&self) -> &Path {
        &self.private_encryption_key_file
    }

    pub fn public_encryption_key(&self) -> &Path {
        &self.public_encryption_key_file
    }

    pub fn ack_key(&self) -> &Path {
        &self.ack_key_file
    }
}

fn file_exists(path: &Path) -> Option<PathBuf> {
    if matches!(path.try_exists(), Ok(true)) {
        return Some(path.to_path_buf());
    }
    None
}
