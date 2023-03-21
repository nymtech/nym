// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::{Config, Socks5, Socks5Debug};
use client_core::config::old_config_v1_1_13::OldConfig_v1_1_13 as OldBaseConfig;
use nym_config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OldConfig_v1_1_13 {
    #[serde(flatten)]
    base: OldBaseConfig<OldConfig_v1_1_13>,

    socks5: Socks5,

    #[serde(default)]
    socks5_debug: Socks5Debug,
}

impl NymConfig for OldConfig_v1_1_13 {
    fn template() -> &'static str {
        // not intended to be used
        unimplemented!()
    }

    fn default_root_directory() -> PathBuf {
        #[cfg(not(feature = "mobile"))]
        let base_dir = dirs::home_dir().expect("Failed to evaluate $HOME value");
        #[cfg(feature = "mobile")]
        let base_dir = PathBuf::from("/tmp");

        base_dir.join(".nym").join("socks5-clients")
    }

    fn try_default_root_directory() -> Option<PathBuf> {
        dirs::home_dir().map(|path| path.join(".nym").join("socks5-clients"))
    }

    fn root_directory(&self) -> PathBuf {
        self.base.client.nym_root_directory.clone()
    }

    fn config_directory(&self) -> PathBuf {
        self.root_directory()
            .join(&self.base.client.id)
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.root_directory()
            .join(&self.base.client.id)
            .join("data")
    }
}

impl From<OldConfig_v1_1_13> for Config {
    fn from(value: OldConfig_v1_1_13) -> Self {
        Config {
            base: value.base.into(),
            socks5: value.socks5,
            socks5_debug: value.socks5_debug,
        }
    }
}
