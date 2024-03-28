// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::disk_persistence::old::v2::{
    ClientKeysPathsV2, CommonClientPathsV2, DEFAULT_GATEWAY_DETAILS_FILENAME,
};
use crate::error::ConfigUpgradeFailure;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// aliases for backwards compatibility
pub type CommonClientPathsV1_1_20_2 = CommonClientPathsV1;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonClientPathsV1 {
    pub keys: ClientKeysPathsV2,
    pub credentials_database: PathBuf,
    pub reply_surb_database: PathBuf,
}

impl CommonClientPathsV1 {
    pub fn upgrade_default(self) -> Result<CommonClientPathsV2, ConfigUpgradeFailure> {
        let data_dir = self
            .reply_surb_database
            .parent()
            .ok_or_else(|| ConfigUpgradeFailure {
                current_version: "1.1.20-2".to_string(),
            })?;
        Ok(CommonClientPathsV2 {
            keys: self.keys,
            gateway_details: data_dir.join(DEFAULT_GATEWAY_DETAILS_FILENAME),
            credentials_database: self.credentials_database,
            reply_surb_database: self.reply_surb_database,
        })
    }
}
