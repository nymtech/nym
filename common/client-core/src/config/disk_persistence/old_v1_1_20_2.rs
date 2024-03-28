// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::old_v1_1_33::{
    ClientKeysPathsV1_1_33, CommonClientPathsV1_1_33, DEFAULT_GATEWAY_DETAILS_FILENAME,
};
use crate::error::ClientCoreError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonClientPathsV1_1_20_2 {
    pub keys: ClientKeysPathsV1_1_33,
    pub credentials_database: PathBuf,
    pub reply_surb_database: PathBuf,
}

impl CommonClientPathsV1_1_20_2 {
    pub fn upgrade_default(self) -> Result<CommonClientPathsV1_1_33, ClientCoreError> {
        let data_dir = self.reply_surb_database.parent().ok_or_else(|| {
            ClientCoreError::ConfigFileUpgradeFailure {
                current_version: "1.1.20-2".to_string(),
            }
        })?;
        Ok(CommonClientPathsV1_1_33 {
            keys: self.keys,
            gateway_details: data_dir.join(DEFAULT_GATEWAY_DETAILS_FILENAME),
            credentials_database: self.credentials_database,
            reply_surb_database: self.reply_surb_database,
        })
    }
}
