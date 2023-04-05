// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use client_core::config::old_config_v1_1_13::OldConfigV1_1_13 as OldBaseConfigV1_1_13;
use nym_config::NymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OldConfigV1_1_13 {
    #[serde(flatten)]
    base: OldBaseConfigV1_1_13<OldConfigV1_1_13>,
}

impl NymConfig for OldConfigV1_1_13 {
    fn template() -> &'static str {
        // not intended to be used
        unimplemented!()
    }

    // TODO: merge base dir with `HostStore`.
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("service-providers")
            .join("network-requester")
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

impl From<OldConfigV1_1_13> for Config {
    fn from(value: OldConfigV1_1_13) -> Self {
        Config {
            base: value.base.into(),
            network_requester: Default::default(),
            network_requester_debug: Default::default(),
        }
    }
}
