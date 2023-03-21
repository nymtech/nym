// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::{Config, Socket};
use client_core::config::old_config_v1_1_13::OldConfig_v1_1_13 as OldBaseConfig;
use nym_config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OldConfig_v1_1_13 {
    #[serde(flatten)]
    base: OldBaseConfig<OldConfig_v1_1_13>,

    socket: Socket,
}

impl NymConfig for OldConfig_v1_1_13 {
    fn template() -> &'static str {
        // not intended to be used
        unimplemented!()
    }

    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("clients")
    }

    fn try_default_root_directory() -> Option<PathBuf> {
        dirs::home_dir().map(|path| path.join(".nym").join("clients"))
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
            socket: value.socket,
        }
    }
}
