// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::NymNodePaths;
use crate::config::template::CONFIG_TEMPLATE;
use crate::error::NymNodeError;
use nym_config::defaults::{DEFAULT_NYM_NODE_HTTP_PORT, WG_PORT};
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use serde_helpers::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use tracing::debug;

pub mod persistence;
mod serde_helpers;
mod template;
mod upgrade_helpers;

const DEFAULT_NYMNODES_DIR: &str = "nym-nodes";

pub const DEFAULT_WIREGUARD_PORT: u16 = WG_PORT;
pub const DEFAULT_WIREGUARD_PREFIX: u8 = 16;
pub const DEFAULT_HTTP_PORT: u16 = DEFAULT_NYM_NODE_HTTP_PORT;

/// Derive default path to nym-node's config directory.
/// It should get resolved to `$HOME/.nym/nym-nodes/<id>/config`
pub fn default_config_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMNODES_DIR)
        .join(id)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to nym-node's config file.
/// It should get resolved to `$HOME/.nym/nym-nodes/<id>/config/config.toml`
pub fn default_config_filepath<P: AsRef<Path>>(id: P) -> PathBuf {
    default_config_directory(id).join(DEFAULT_CONFIG_FILENAME)
}

/// Derive default path to nym-node's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/nym-nodes/<id>/data`
pub fn default_data_directory<P: AsRef<Path>>(id: P) -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYMNODES_DIR)
        .join(id)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    pub id: String,

    pub storage_paths: NymNodePaths,
}

// from<config> for gateway_config and from<config> for mixnode_config

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn save(&self) -> Result<(), NymNodeError> {
        let save_location = self.save_location();
        debug!(
            "attempting to save config file to '{}'",
            save_location.display()
        );
        save_formatted_config_to_file(self, &save_location).map_err(|source| {
            NymNodeError::ConfigSaveFailure {
                id: self.id.clone(),
                path: save_location,
                source,
            }
        })
    }

    pub fn save_location(&self) -> PathBuf {
        self.save_path
            .clone()
            .unwrap_or(self.default_save_location())
    }

    pub fn default_save_location(&self) -> PathBuf {
        default_config_filepath(&self.id)
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        let path = path.as_ref();
        let mut loaded: Config =
            read_config_from_toml_file(path).map_err(|source| NymNodeError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            })?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, NymNodeError> {
        Self::read_from_path(path)
    }
}

// TODO: this is very much a WIP. we need proper ssl certificate support here
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    /// Ip address(es) of this host, such as 1.1.1.1 that external clients will use for connections.
    pub public_ips: Vec<IpAddr>,

    /// Optional hostname of this node, for example nymtech.net.
    // TODO: this is temporary. to be replaced by pulling the data directly from the certs.
    #[serde(deserialize_with = "de_maybe_string")]
    pub hostname: Option<String>,
}

impl Host {
    pub fn validate(&self) -> bool {
        if self.public_ips.is_empty() {
            return false;
        }

        true
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Http {
    /// Socket address this node will use for binding its http API.
    /// default: `0.0.0.0:8080`
    /// note: for legacy reasons, it defaults to port `8000` for mixnodes.
    pub bind_address: SocketAddr,

    /// Path to assets directory of custom landing page of this node.
    #[serde(deserialize_with = "de_maybe_path")]
    pub landing_page_assets_path: Option<PathBuf>,

    #[serde(default)]
    pub metrics_key: Option<String>,
}

impl Default for Http {
    fn default() -> Self {
        Http {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DEFAULT_HTTP_PORT),
            landing_page_assets_path: None,
            metrics_key: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Wireguard {
    /// Specifies whether the wireguard service is enabled on this node.
    pub enabled: bool,

    /// Socket address this node will use for binding its wireguard interface.
    /// default: `0.0.0.0:51822`
    pub bind_address: SocketAddr,

    /// Port announced to external clients wishing to connect to the wireguard interface.
    /// Useful in the instances where the node is behind a proxy.
    pub announced_port: u16,

    /// The prefix denoting the maximum number of the clients that can be connected via Wireguard.
    /// The maximum value for IPv4 is 32 and for IPv6 is 128
    pub private_network_prefix: u8,

    /// Paths for wireguard keys, client registries, etc.
    pub storage_paths: persistence::WireguardPaths,
}

impl Default for Wireguard {
    fn default() -> Self {
        Wireguard {
            enabled: false,
            bind_address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                DEFAULT_WIREGUARD_PORT,
            ),
            announced_port: DEFAULT_WIREGUARD_PORT,
            private_network_prefix: DEFAULT_WIREGUARD_PREFIX,
            storage_paths: persistence::WireguardPaths {},
        }
    }
}
