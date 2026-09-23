// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::disk_persistence::{
    ClientKeysPaths, CommonClientPaths, DEFAULT_CREDENTIAL_REQUESTS_DB_FILENAME,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The common paths as they were stored before `credential_requests_database` was introduced.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct CommonClientPathsV3 {
    pub keys: ClientKeysPaths,

    /// Path to the file containing information about gateways used by this client,
    /// i.e. details such as their public keys, owner addresses or the network information.
    pub gateway_registrations: PathBuf,

    /// Path to the database containing bandwidth credentials of this client.
    pub credentials_database: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,
}

impl CommonClientPathsV3 {
    pub fn upgrade(self) -> CommonClientPaths {
        // all the stores sit side by side in the data directory,
        // so it can be recovered from any of the existing entries
        let data_dir = self
            .credentials_database
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();

        CommonClientPaths {
            keys: self.keys,
            gateway_registrations: self.gateway_registrations,
            credentials_database: self.credentials_database,
            credential_requests_database: data_dir.join(DEFAULT_CREDENTIAL_REQUESTS_DB_FILENAME),
            reply_surb_database: self.reply_surb_database,
        }
    }
}
