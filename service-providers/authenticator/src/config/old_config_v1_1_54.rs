use std::{io, path::Path};

use nym_bin_common::logging::LoggingSettings;
pub use nym_client_core::config::old_config_v1_1_54::ConfigV1_1_54 as BaseConfigV1_1_54;
use nym_config::read_config_from_toml_file;
use serde::{Deserialize, Serialize};

use crate::{config::Config, error::AuthenticatorError};

use super::{default_config_filepath, Authenticator, AuthenticatorPaths};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_54 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_54,

    #[serde(default)]
    pub authenticator: Authenticator,

    pub storage_paths: AuthenticatorPaths,

    pub logging: LoggingSettings,
}

impl ConfigV1_1_54 {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn read_from_default_path<P: AsRef<Path>>(id: P) -> io::Result<Self> {
        Self::read_from_toml_file(default_config_filepath(id))
    }

    pub fn try_upgrade(self) -> Result<Config, AuthenticatorError> {
        Ok(Config {
            base: self.base.into(),
            authenticator: self.authenticator,
            storage_paths: self.storage_paths,
            logging: self.logging,
        })
    }
}
