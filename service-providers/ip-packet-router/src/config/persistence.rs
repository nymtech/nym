// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_client_core::config::ConfigUpgradeFailure;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_client_core::config::disk_persistence::old::v3::CommonClientPathsV3;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_DESCRIPTION_FILENAME: &str = "description.toml";

/// The paths as they were stored before `credential_requests_database` was introduced.
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct IpPacketRouterPathsV2 {
    #[serde(flatten)]
    pub common_paths: CommonClientPathsV3,

    /// Location of the file containing our description
    pub ip_packet_router_description: PathBuf,
}

impl IpPacketRouterPathsV2 {
    pub fn upgrade(self) -> Result<IpPacketRouterPaths, ConfigUpgradeFailure> {
        Ok(IpPacketRouterPaths {
            common_paths: self.common_paths.upgrade()?,
            ip_packet_router_description: self.ip_packet_router_description,
        })
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize, Clone)]
pub struct IpPacketRouterPaths {
    #[serde(flatten)]
    pub common_paths: CommonClientPaths,

    /// Location of the file containing our description
    pub ip_packet_router_description: PathBuf,
}

impl IpPacketRouterPaths {
    pub fn new_base<P: AsRef<Path>>(base_data_directory: P) -> Self {
        let base_dir = base_data_directory.as_ref();

        Self {
            common_paths: CommonClientPaths::new_base(base_dir),
            ip_packet_router_description: base_dir.join(DEFAULT_DESCRIPTION_FILENAME),
        }
    }
}
