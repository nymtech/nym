use std::{io, path::Path};

use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::old_config_v1_1_54::ConfigV1_1_54 as BaseConfigV1_1_54;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};

use crate::error::ClientError;

use super::{default_config_filepath, persistence::ClientPaths, Config, Socket};

#[derive(Debug, Deserialize, PartialEq, Serialize, Clone)]
pub struct ConfigV1_1_54 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_54,

    pub socket: Socket,

    pub storage_paths: ClientPaths,

    pub logging: LoggingSettings,
}

impl ConfigV1_1_54 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<Config, ClientError> {
        Ok(Config {
            base: self.base.into(),
            socket: self.socket,
            storage_paths: self.storage_paths,
            logging: self.logging,
        })
    }
}
