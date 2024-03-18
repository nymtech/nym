// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::ExitGatewayPaths;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayConfig {
    pub storage_paths: ExitGatewayPaths,

    pub network_requester: NetworkRequester,

    pub ip_packet_router: IpPacketRouter,
}

impl ExitGatewayConfig {
    pub fn new_default<P: AsRef<Path>>(config_dir: P) -> Self {
        ExitGatewayConfig {
            storage_paths: ExitGatewayPaths::new(config_dir),
            network_requester: Default::default(),
            ip_packet_router: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct NetworkRequester {
    /// Specifies whether network requester service is enabled in this process.
    pub enabled: bool,
    // TODO: all NR things should eventually live here
}

#[allow(clippy::derivable_impls)]
impl Default for NetworkRequester {
    fn default() -> Self {
        NetworkRequester { enabled: false }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct IpPacketRouter {
    /// Specifies whether ip packet router service is enabled in this process.
    pub enabled: bool,
    // TODO: all IPR things should eventually live here
}

#[allow(clippy::derivable_impls)]
impl Default for IpPacketRouter {
    fn default() -> Self {
        Self { enabled: false }
    }
}
