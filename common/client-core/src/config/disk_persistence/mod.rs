// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::keys_paths::ClientKeysPaths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod keys_paths;
pub mod old_v1_1_20_2;
mod old_v1_1_33;

pub const DEFAULT_GATEWAY_DETAILS_FILENAME: &str = "gateway_details.json";
pub const DEFAULT_REPLY_SURB_DB_FILENAME: &str = "persistent_reply_store.sqlite";
pub const DEFAULT_CREDENTIALS_DB_FILENAME: &str = "credentials_database.db";
pub const DEFAULT_GATEWAYS_DETAILS_DB_FILENAME: &str = "gateways_registrations.db";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommonClientPaths {
    pub keys: ClientKeysPaths,

    /// Path to the file containing information about gateway used by this client,
    /// i.e. details such as its public key, owner address or the network information.
    #[deprecated]
    pub gateway_details: PathBuf,

    // TODO:
    pub gateway_registrations: PathBuf,

    /// Path to the database containing bandwidth credentials of this client.
    pub credentials_database: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,
}

impl CommonClientPaths {
    pub fn new_base<P: AsRef<Path>>(base_data_directory: P) -> Self {
        let base_dir = base_data_directory.as_ref();

        CommonClientPaths {
            credentials_database: base_dir.join(DEFAULT_CREDENTIALS_DB_FILENAME),
            reply_surb_database: base_dir.join(DEFAULT_REPLY_SURB_DB_FILENAME),
            gateway_details: base_dir.join(DEFAULT_GATEWAY_DETAILS_FILENAME),
            gateway_registrations: base_dir.join(DEFAULT_GATEWAYS_DETAILS_DB_FILENAME),
            keys: ClientKeysPaths::new_base(base_data_directory),
        }
    }
}
