use std::{io, path::Path};

use crate::{config::Config, error::IpPacketRouterError};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::old_config_v1_1_54::ConfigV1_1_54 as BaseConfigV1_1_54;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};

use super::{default_config_filepath, IpPacketRouter, IpPacketRouterPaths};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV2 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_54,

    #[serde(default)]
    pub ip_packet_router: IpPacketRouter,

    pub storage_paths: IpPacketRouterPaths,

    pub logging: LoggingSettings,
}

impl ConfigV2 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<Config, IpPacketRouterError> {
        Ok(Config {
            base: self.base.into(),
            ip_packet_router: self.ip_packet_router,
            storage_paths: self.storage_paths,
            logging: self.logging,
        })
    }
}
