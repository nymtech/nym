// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_data_directory;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME: &str = "private_identity.pem";
pub const DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME: &str = "public_identity.pem";
pub const DEFAULT_PRIVATE_SPHINX_KEY_FILENAME: &str = "private_sphinx.pem";
pub const DEFAULT_PUBLIC_SPHINX_KEY_FILENAME: &str = "public_sphinx.pem";

pub const DEFAULT_CLIENTS_STORAGE_FILENAME: &str = "db.sqlite";

// pub const DEFAULT_DESCRIPTION_FILENAME: &str = "description.toml";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayPaths {
    pub keys: KeysPaths,

    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys and available client bandwidths.
    #[serde(alias = "persistent_storage")]
    pub clients_storage: PathBuf,
    // pub node_description: PathBuf,

    // pub cosmos_bip39_mnemonic: PathBuf,
}

impl GatewayPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        GatewayPaths {
            keys: KeysPaths::new_default(id.as_ref()),
            clients_storage: default_data_directory(id).join(DEFAULT_CLIENTS_STORAGE_FILENAME),
            // node_description: default_config_filepath(id).join(DEFAULT_DESCRIPTION_FILENAME),
        }
    }

    pub fn private_identity_key(&self) -> &Path {
        self.keys.private_identity_key()
    }

    pub fn public_identity_key(&self) -> &Path {
        self.keys.public_identity_key()
    }

    pub fn private_encryption_key(&self) -> &Path {
        self.keys.private_encryption_key()
    }

    pub fn public_encryption_key(&self) -> &Path {
        self.keys.public_encryption_key()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct KeysPaths {
    /// Path to file containing private identity key.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    pub public_identity_key_file: PathBuf,

    /// Path to file containing private sphinx key.
    pub private_sphinx_key_file: PathBuf,

    /// Path to file containing public sphinx key.
    pub public_sphinx_key_file: PathBuf,
}

impl KeysPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        KeysPaths {
            private_identity_key_file: data_dir.join(DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME),
            public_identity_key_file: data_dir.join(DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME),
            private_sphinx_key_file: data_dir.join(DEFAULT_PRIVATE_SPHINX_KEY_FILENAME),
            public_sphinx_key_file: data_dir.join(DEFAULT_PUBLIC_SPHINX_KEY_FILENAME),
        }
    }

    pub fn private_identity_key(&self) -> &Path {
        &self.private_identity_key_file
    }

    pub fn public_identity_key(&self) -> &Path {
        &self.public_identity_key_file
    }

    pub fn private_encryption_key(&self) -> &Path {
        &self.private_sphinx_key_file
    }

    pub fn public_encryption_key(&self) -> &Path {
        &self.public_sphinx_key_file
    }
}
