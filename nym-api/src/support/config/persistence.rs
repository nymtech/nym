// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config::default_data_directory;
use anyhow::Context;
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_NETWORK_MONITOR_CREDENTIALS_DATABASE_FILENAME: &str = "credentials_database.db";

pub const DEFAULT_NODE_STATUS_API_DATABASE_FILENAME: &str = "db.sqlite";

pub const DEFAULT_DKG_PERSISTENT_STATE_FILENAME: &str = "dkg_persistent_state.json";
pub const DEFAULT_DKG_DECRYPTION_KEY_FILENAME: &str = "dkg_decryption_key.pem";
pub const DEFAULT_DKG_PUBLIC_KEY_WITH_PROOF_FILENAME: &str = "dkg_public_key_with_proof.pem";

// don't want to be changing the defaults in case something breaks..., but it should be called ecash.pem instead
pub const DEFAULT_ECASH_KEY_FILENAME: &str = "coconut.pem";

pub const DEFAULT_CACHES_DIRECTORY: &str = ".cache";
pub const DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME: &str = "private_identity.pem";
pub const DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME: &str = "public_identity.pem";

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct NetworkMonitorPaths {
    // TODO: this should contain the path to the database holding the results, but changing it would break backwards compatibility
    /// Path to the database containing bandwidth credentials of this client.
    pub credentials_database_path: PathBuf,
}

impl NetworkMonitorPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        NetworkMonitorPaths {
            credentials_database_path: data_dir
                .join(DEFAULT_NETWORK_MONITOR_CREDENTIALS_DATABASE_FILENAME),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct NodeStatusAPIPaths {
    /// Path to the database file containing uptime statuses for all mixnodes and gateways.
    pub database_path: PathBuf,
}

impl NodeStatusAPIPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        NodeStatusAPIPaths {
            database_path: data_dir.join(DEFAULT_NODE_STATUS_API_DATABASE_FILENAME),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct EcashSignerPaths {
    /// Path to a JSON file where state is persisted between different stages of DKG.
    pub dkg_persistent_state_path: PathBuf,

    /// Path to the coconut key.
    #[serde(alias = "coconut_key_path")]
    pub ecash_key_path: PathBuf,

    /// Path to the dkg dealer decryption key.
    pub decryption_key_path: PathBuf,

    /// Path to the dkg dealer public key with proof.
    pub public_key_with_proof_path: PathBuf,
}

impl EcashSignerPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        EcashSignerPaths {
            dkg_persistent_state_path: data_dir.join(DEFAULT_DKG_PERSISTENT_STATE_FILENAME),
            ecash_key_path: data_dir.join(DEFAULT_ECASH_KEY_FILENAME),
            decryption_key_path: data_dir.join(DEFAULT_DKG_DECRYPTION_KEY_FILENAME),
            public_key_with_proof_path: data_dir.join(DEFAULT_DKG_PUBLIC_KEY_WITH_PROOF_FILENAME),
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NymApiPaths {
    /// Path to directory containing persistent caches of, for example,
    /// the describe information, performance, etc.
    /// It is used for restarting the nym-api and preserving the data
    pub persistent_cache_directory: PathBuf,

    /// Path to file containing private identity key of the nym-api.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key of the nym-api.
    pub public_identity_key_file: PathBuf,
}

impl NymApiPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        NymApiPaths {
            persistent_cache_directory: data_dir.join(DEFAULT_CACHES_DIRECTORY),
            private_identity_key_file: data_dir.join(DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME),
            public_identity_key_file: data_dir.join(DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME),
        }
    }

    pub fn cache_file(&self, name: impl AsRef<Path>) -> PathBuf {
        self.persistent_cache_directory.join(name)
    }

    pub fn load_identity(&self) -> anyhow::Result<ed25519::KeyPair> {
        let keypaths = nym_pemstore::KeyPairPath::new(
            &self.private_identity_key_file,
            &self.public_identity_key_file,
        );

        nym_pemstore::load_keypair(&keypaths).context(format!(
            "failed to load identity keys of the nym api. paths: {keypaths:?}"
        ))
    }
}
