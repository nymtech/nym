// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::keys_paths::ClientKeysPaths;
use crate::config::disk_persistence::{CommonClientPaths, DEFAULT_GATEWAYS_DETAILS_DB_FILENAME};
use crate::error::ClientCoreError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_GATEWAY_DETAILS_FILENAME: &str = "gateway_details.json";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct ClientKeysPathsV1_1_33 {
    /// Path to file containing private identity key.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    pub public_identity_key_file: PathBuf,

    /// Path to file containing private encryption key.
    pub private_encryption_key_file: PathBuf,

    /// Path to file containing public encryption key.
    pub public_encryption_key_file: PathBuf,

    /// Path to file containing shared key derived with the specified gateway that is used
    /// for all communication with it.
    pub gateway_shared_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,
}

impl ClientKeysPathsV1_1_33 {
    pub fn upgrade(self) -> ClientKeysPaths {
        ClientKeysPaths {
            private_identity_key_file: self.private_identity_key_file,
            public_identity_key_file: self.public_identity_key_file,
            private_encryption_key_file: self.private_encryption_key_file,
            public_encryption_key_file: self.public_encryption_key_file,
            ack_key_file: self.ack_key_file,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonClientPathsV1_1_33 {
    pub keys: ClientKeysPathsV1_1_33,

    /// Path to the file containing information about gateway used by this client,
    /// i.e. details such as its public key, owner address or the network information.
    pub gateway_details: PathBuf,

    /// Path to the database containing bandwidth credentials of this client.
    pub credentials_database: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,
}

impl CommonClientPathsV1_1_33 {
    // note that during the upgrade process, the caller will need to extract the key and gateway details
    // manually and resave them in the new database
    pub fn upgrade_default(self) -> Result<CommonClientPaths, ClientCoreError> {
        let data_dir = self.gateway_details.parent().ok_or_else(|| {
            ClientCoreError::ConfigFileUpgradeFailure {
                current_version: "1.1.33".to_string(),
            }
        })?;

        Ok(CommonClientPaths {
            keys: self.keys.upgrade(),
            gateway_registrations: data_dir.join(DEFAULT_GATEWAYS_DETAILS_DB_FILENAME),
            credentials_database: self.credentials_database,
            reply_surb_database: self.reply_surb_database,
        })
    }
}
