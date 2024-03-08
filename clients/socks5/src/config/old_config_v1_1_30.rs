// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::default_config_filepath;
use crate::config::old_config_v1_1_33::{ConfigV1_1_33, SocksClientPathsV1_1_33};
use nym_bin_common::logging::LoggingSettings;
use nym_config::read_config_from_toml_file;
use nym_socks5_client_core::config::old_config_v1_1_30::ConfigV1_1_30 as CoreConfigV1_1_30;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_30 {
    pub core: CoreConfigV1_1_30,

    pub storage_paths: SocksClientPathsV1_1_33,

    pub logging: LoggingSettings,
}

impl From<ConfigV1_1_30> for ConfigV1_1_33 {
    fn from(value: ConfigV1_1_30) -> Self {
        ConfigV1_1_33 {
            core: value.core.into(),
            storage_paths: value.storage_paths,
            logging: LoggingSettings::default(),
        }
    }
}

impl ConfigV1_1_30 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }
}
