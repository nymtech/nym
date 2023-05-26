// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::config::default_data_directory;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_NETWORK_MONITOR_CREDENTIALS_DATABASE_FILENAME: &str = "credentials_database.db";

pub const DEFAULT_NODE_STATUS_API_DATABASE_FILENAME: &str = "db.sqlite";

pub const DEFAULT_DKG_PERSISTENT_STATE_FILENAME: &str = "dkg_persistent_state.json";
pub const DEFAULT_DKG_DECRYPTION_KEY_FILENAME: &str = "dkg_decryption_key.pem";
pub const DEFAULT_DKG_PUBLIC_KEY_WITH_PROOF_FILENAME: &str = "dkg_public_key_with_proof.pem";
pub const DEFAULT_COCONUT_VERIFICATION_KEY_FILENAME: &str = "coconut_verification_key.pem";
pub const DEFAULT_COCONUT_SECRET_KEY_FILENAME: &str = "coconut_secret_key.pem";

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
pub struct NetworkMonitorPathfinder {
    // TODO: this should contain the path to the database holding the results, but changing it would break backwards compatibility
    /// Path to the database containing bandwidth credentials of this client.
    pub credentials_database_path: PathBuf,
}

impl NetworkMonitorPathfinder {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        NetworkMonitorPathfinder {
            credentials_database_path: data_dir
                .join(DEFAULT_NETWORK_MONITOR_CREDENTIALS_DATABASE_FILENAME),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct NodeStatusAPIPathfinder {
    /// Path to the database file containing uptime statuses for all mixnodes and gateways.
    pub database_path: PathBuf,
}

impl NodeStatusAPIPathfinder {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        NodeStatusAPIPathfinder {
            database_path: data_dir.join(DEFAULT_NODE_STATUS_API_DATABASE_FILENAME),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct CoconutSignerPathfinder {
    /// Path to a JSON file where state is persisted between different stages of DKG.
    pub dkg_persistent_state_path: PathBuf,

    /// Path to the coconut verification key.
    pub verification_key_path: PathBuf,

    /// Path to the coconut secret key.
    pub secret_key_path: PathBuf,

    /// Path to the dkg dealer decryption key.
    pub decryption_key_path: PathBuf,

    /// Path to the dkg dealer public key with proof.
    pub public_key_with_proof_path: PathBuf,
}

impl CoconutSignerPathfinder {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        CoconutSignerPathfinder {
            dkg_persistent_state_path: data_dir.join(DEFAULT_DKG_PERSISTENT_STATE_FILENAME),
            verification_key_path: data_dir.join(DEFAULT_COCONUT_VERIFICATION_KEY_FILENAME),
            secret_key_path: data_dir.join(DEFAULT_COCONUT_SECRET_KEY_FILENAME),
            decryption_key_path: data_dir.join(DEFAULT_DKG_DECRYPTION_KEY_FILENAME),
            public_key_with_proof_path: data_dir.join(DEFAULT_DKG_PUBLIC_KEY_WITH_PROOF_FILENAME),
        }
    }
}
