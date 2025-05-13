use std::{io, path::Path};

use nym_bin_common::logging::LoggingSettings;
use nym_config::read_config_from_toml_file;
use nym_socks5_client_core::config::old_config_v1_1_54::ConfigV1_1_54 as CoreConfigV1_1_54;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::Socks5ClientError;

use super::{default_config_filepath, SocksClientPaths};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_54 {
    pub core: CoreConfigV1_1_54,

    pub storage_paths: SocksClientPaths,

    pub logging: LoggingSettings,
}

impl ConfigV1_1_54 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<Config, Socks5ClientError> {
        Ok(Config {
            core: self.core.into(),
            storage_paths: self.storage_paths,
            logging: self.logging,
        })
    }
}
