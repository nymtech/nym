use std::{io, path::Path};

use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::Config as BaseClientConfig;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};

use crate::config::{
    default_config_filepath, Config, Debug, NetworkRequester, NetworkRequesterPaths,
};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV6 {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    #[serde(default)]
    pub network_requester: NetworkRequester,

    pub storage_paths: NetworkRequesterPaths,

    #[serde(default)]
    pub network_requester_debug: Debug,

    pub logging: LoggingSettings,
}

impl ConfigV6 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    #[allow(dead_code)]
    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }
}

impl From<ConfigV6> for Config {
    fn from(value: ConfigV6) -> Self {
        Config {
            base: value.base,
            network_requester: value.network_requester,
            storage_paths: value.storage_paths,
            network_requester_debug: value.network_requester_debug,
            logging: value.logging,
        }
    }
}
