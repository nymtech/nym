// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::keys_paths::{
    ClientKeysPaths, DEFAULT_PRIVATE_ECASH_KEY_FILENAME, DEFAULT_PUBLIC_ECASH_KEY_FILENAME,
};
use crate::config::disk_persistence::{CommonClientPaths, DEFAULT_GATEWAY_DETAILS_FILENAME};
use crate::error::ClientCoreError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonClientPathsV1_1_20_2 {
    pub keys: ClientKeysPathsV1_1_20_2,
    pub credentials_database: PathBuf,
    pub reply_surb_database: PathBuf,
}

impl CommonClientPathsV1_1_20_2 {
    pub fn upgrade_default(self) -> Result<CommonClientPaths, ClientCoreError> {
        let data_dir = self.reply_surb_database.parent().ok_or_else(|| {
            ClientCoreError::UnableToUpgradeConfigFile {
                new_version: "1.1.20-2".to_string(),
            }
        })?;
        Ok(CommonClientPaths {
            keys: self.keys.upgrade_default()?,
            gateway_details: data_dir.join(DEFAULT_GATEWAY_DETAILS_FILENAME),
            credentials_database: self.credentials_database,
            reply_surb_database: self.reply_surb_database,
        })
    }
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClientKeysPathsV1_1_20_2 {
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

impl ClientKeysPathsV1_1_20_2 {
    pub fn upgrade_default(self) -> Result<ClientKeysPaths, ClientCoreError> {
        let data_dir = self.gateway_shared_key_file.parent().ok_or_else(|| {
            ClientCoreError::UnableToUpgradeConfigFile {
                new_version: "1.1.20-2".to_string(),
            }
        })?;
        Ok(ClientKeysPaths {
            private_identity_key_file: self.private_identity_key_file,
            public_identity_key_file: self.public_identity_key_file,
            private_encryption_key_file: self.private_encryption_key_file,
            public_encryption_key_file: self.public_encryption_key_file,
            private_ecash_key_file: data_dir.join(DEFAULT_PRIVATE_ECASH_KEY_FILENAME),
            public_ecash_key_file: data_dir.join(DEFAULT_PUBLIC_ECASH_KEY_FILENAME),
            gateway_shared_key_file: self.gateway_shared_key_file,
            ack_key_file: self.ack_key_file,
        })
    }
}
