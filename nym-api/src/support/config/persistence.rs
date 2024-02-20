// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::config::default_data_directory;
use anyhow::Context;
use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_NETWORK_MONITOR_CREDENTIALS_DATABASE_FILENAME: &str = "credentials_database.db";

pub const DEFAULT_NODE_STATUS_API_DATABASE_FILENAME: &str = "db.sqlite";

pub const DEFAULT_DKG_PERSISTENT_STATE_FILENAME: &str = "dkg_persistent_state.json";
pub const DEFAULT_DKG_DECRYPTION_KEY_FILENAME: &str = "dkg_decryption_key.pem";
pub const DEFAULT_DKG_PUBLIC_KEY_WITH_PROOF_FILENAME: &str = "dkg_public_key_with_proof.pem";
pub const DEFAULT_COCONUT_KEY_FILENAME: &str = "coconut.pem";

pub const DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME: &str = "private_identity.pem";
pub const DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME: &str = "public_identity.pem";

// #[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
// pub struct NymApiPathfinder {
//     pub network_monitor: NetworkMonitorPathfinder,
//
//     pub node_status_api: NodeStatusAPIPathfinder,
//
//     pub coconut: CoconutSignerPathfinder,
// }
//
// impl NymApiPathfinder {
//     pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
//         NymApiPathfinder {
//             network_monitor: NetworkMonitorPathfinder::new_default(id.as_ref()),
//             node_status_api: NodeStatusAPIPathfinder::new_default(id.as_ref()),
//             coconut: CoconutSignerPathfinder::new_default(id.as_ref()),
//         }
//     }
// }

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
pub struct CoconutSignerPaths {
    /// Path to a JSON file where state is persisted between different stages of DKG.
    pub dkg_persistent_state_path: PathBuf,

    /// Path to the coconut key.
    pub coconut_key_path: PathBuf,

    /// Path to the dkg dealer decryption key.
    pub decryption_key_path: PathBuf,

    /// Path to the dkg dealer public key with proof.
    pub public_key_with_proof_path: PathBuf,
}

impl CoconutSignerPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        CoconutSignerPaths {
            dkg_persistent_state_path: data_dir.join(DEFAULT_DKG_PERSISTENT_STATE_FILENAME),
            coconut_key_path: data_dir.join(DEFAULT_COCONUT_KEY_FILENAME),
            decryption_key_path: data_dir.join(DEFAULT_DKG_DECRYPTION_KEY_FILENAME),
            public_key_with_proof_path: data_dir.join(DEFAULT_DKG_PUBLIC_KEY_WITH_PROOF_FILENAME),
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct NymApiPaths {
    /// Path to file containing private identity key of the nym-api.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key of the nym-api.
    pub public_identity_key_file: PathBuf,
}

impl NymApiPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        NymApiPaths {
            private_identity_key_file: data_dir.join(DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME),
            public_identity_key_file: data_dir.join(DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME),
        }
    }

    pub fn load_identity(&self) -> anyhow::Result<identity::KeyPair> {
        let keypaths = nym_pemstore::KeyPairPath::new(
            &self.private_identity_key_file,
            &self.public_identity_key_file,
        );

        nym_pemstore::load_keypair(&keypaths).context(format!(
            "failed to load identity keys of the nym api. paths: {keypaths:?}"
        ))
    }
}
