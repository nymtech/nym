// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::key_pathfinder::ClientKeysPathfinder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod key_pathfinder;

pub const DEFAULT_REPLY_SURB_DB_FILENAME: &str = "persistent_reply_store.sqlite";
pub const DEFAULT_CREDENTIALS_DB_FILENAME: &str = "credentials_database.db";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct CommonClientPathfinder {
    pub key_pathfinder: ClientKeysPathfinder,

    // TODO:
    // pub gateway_config_pathfinder: (),
    
    /// Path to the database containing bandwidth credentials of this client.
    #[serde(alias = "database_path")]
    pub credentials_database: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database_path: PathBuf,
}

impl CommonClientPathfinder {
    pub fn new_default<P: AsRef<Path>>(base_data_directory: P) -> Self {
        let base_dir = base_data_directory.as_ref();

        CommonClientPathfinder {
            credentials_database: base_dir.join(DEFAULT_CREDENTIALS_DB_FILENAME),
            reply_surb_database_path: base_dir.join(DEFAULT_REPLY_SURB_DB_FILENAME),
            key_pathfinder: ClientKeysPathfinder::new_default(base_data_directory),
        }
    }
}
