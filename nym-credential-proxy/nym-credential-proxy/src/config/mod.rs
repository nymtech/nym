// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_config::{must_get_home, DEFAULT_DATA_DIR, NYM_DIR};
use std::path::PathBuf;

pub const DEFAULT_NYM_CREDENTIAL_PROXY_DIR: &str = "nym-credential-proxy";

pub const DEFAULT_DB_FILENAME: &str = "nym-credential-proxy.sqlite";

pub fn default_data_directory() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYM_CREDENTIAL_PROXY_DIR)
        .join(DEFAULT_DATA_DIR)
}

pub fn default_database_filepath() -> PathBuf {
    default_data_directory().join(DEFAULT_DB_FILENAME)
}
