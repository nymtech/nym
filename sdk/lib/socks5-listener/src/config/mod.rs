// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::persistence::MobileSocksClientPaths;
use crate::config::template::CONFIG_TEMPLATE;
use nym_bin_common::logging::LoggingSettings;
use nym_config_common::{
    read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_socks5_client_core::config::Config as CoreConfig;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

mod persistence;
mod template;

const DEFAULT_SOCKS5_CLIENTS_DIR: &str = "socks5-clients";

/// Derive default path to clients's config directory.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config`
pub fn config_directory_from_root<P: AsRef<Path>, R: AsRef<Path>>(root: P, id: R) -> PathBuf {
    root.as_ref()
        .join(NYM_DIR)
        .join(DEFAULT_SOCKS5_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to client's config file.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/config/config.toml`
pub fn config_filepath_from_root<P: AsRef<Path>, R: AsRef<Path>>(root: P, id: R) -> PathBuf {
    config_directory_from_root(root, id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to client's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/socks5-clients/<id>/data`
pub fn data_directory_from_root<P: AsRef<Path>, R: AsRef<Path>>(root: P, id: R) -> PathBuf {
    root.as_ref()
        .join(NYM_DIR)
        .join(DEFAULT_SOCKS5_CLIENTS_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub core: CoreConfig,

    pub storage_paths: Option<MobileSocksClientPaths>,

    pub logging: LoggingSettings,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new<P, S>(storage_root: Option<P>, id: S, provider_mix_address: S) -> Self
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        Config {
            core: CoreConfig::new(
                id.as_ref(),
                env!("CARGO_PKG_VERSION"),
                provider_mix_address.as_ref(),
            ),
            storage_paths: storage_root.map(|storage_root| {
                MobileSocksClientPaths::new_default(data_directory_from_root(
                    storage_root,
                    id.as_ref(),
                ))
            }),
            logging: Default::default(),
        }
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>, R: AsRef<Path>>(
        storage_root: P,
        id: R,
    ) -> io::Result<Self> {
        Self::read_from_toml_file(config_filepath_from_root(storage_root, id))
    }

    pub fn save_to_default_location<P: AsRef<Path>>(&self, storage_root: P) -> io::Result<()> {
        let config_save_location: PathBuf =
            config_filepath_from_root(storage_root, &self.core.base.client.id);
        save_formatted_config_to_file(self, config_save_location)
    }
}
