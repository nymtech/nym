// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

fn de_maybe_path<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    let path = PathBuf::deserialize(deserializer)?;
    if path.as_os_str().is_empty() {
        Ok(None)
    } else {
        Ok(Some(path))
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Http {
    /// Socket address this node will use for binding its http API.
    /// default: `0.0.0.0:80`
    pub bind_address: SocketAddr,

    /// Path to assets directory of custom landing page of this node.
    #[serde(deserialize_with = "de_maybe_path")]
    pub landing_page_assets_path: Option<PathBuf>,
}

impl Default for Http {
    fn default() -> Self {
        Http {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 80),
            landing_page_assets_path: None,
        }
    }
}
