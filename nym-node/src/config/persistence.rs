// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_identity";
pub const DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_identity.pub";
pub const DEFAULT_X25519_PRIVATE_SPHINX_KEY_FILENAME: &str = "x25519_sphinx";
pub const DEFAULT_X25519_PUBLIC_SPHINX_KEY_FILENAME: &str = "x25519_sphinx.pub";

#[derive(Debug, Serialize, Deserialize)]
pub struct NymNodePaths {
    pub keys: KeysPaths,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct KeysPaths {
    /// Path to file containing ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing x25519 sphinx private key.
    pub private_x25519_sphinx_key_file: PathBuf,

    /// Path to file containing x25519 sphinx public key.
    pub public_x25519_sphinx_key_file: PathBuf,
}

impl KeysPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();

        KeysPaths {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_sphinx_key_file: data_dir
                .join(DEFAULT_X25519_PRIVATE_SPHINX_KEY_FILENAME),
            public_x25519_sphinx_key_file: data_dir.join(DEFAULT_X25519_PUBLIC_SPHINX_KEY_FILENAME),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.public_ed25519_identity_key_file,
            &self.private_ed25519_identity_key_file,
        )
    }

    pub fn x25519_sphinx_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.public_x25519_sphinx_key_file,
            &self.private_x25519_sphinx_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardPaths {
    // pub keys:
}
