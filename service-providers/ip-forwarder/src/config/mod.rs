use std::{io, path::Path};

pub use nym_client_core::config::Config as BaseClientConfig;
use serde::{Deserialize, Serialize};

use crate::config::persistence::IpForwarderPaths;

mod persistence;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    pub storage_paths: IpForwarderPaths,
}

impl Config {
    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        nym_config::read_config_from_toml_file(path)
    }
}
