// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::keys_paths::ClientKeysPaths;
use crate::config::disk_persistence::{CommonClientPaths, DEFAULT_GATEWAY_DETAILS_FILENAME};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonClientPathsV1_1_20_2 {
    pub keys: ClientKeysPaths,
    pub credentials_database: PathBuf,
    pub reply_surb_database: PathBuf,
}

impl CommonClientPathsV1_1_20_2 {
    pub fn upgrade_default(self) -> CommonClientPaths {
        let data_dir = self
            .reply_surb_database
            .parent()
            .expect("client paths upgrade failure");
        CommonClientPaths {
            keys: self.keys,
            gateway_details: data_dir.join(DEFAULT_GATEWAY_DETAILS_FILENAME),
            credentials_database: self.credentials_database,
            reply_surb_database: self.reply_surb_database,
        }
    }
}
