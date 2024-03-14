// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::entry_gateway::EntryGatewayConfig;
use crate::config::exit_gateway::ExitGatewayConfig;
use crate::config::mixnode::MixnodeConfig;
use crate::config::persistence::NymNodePaths;
use crate::config::template::CONFIG_TEMPLATE;
use crate::error::NymNodeError;
use clap::ValueEnum;
use nym_bin_common::logging::LoggingSettings;
use nym_config::defaults::{
    mainnet, var_names, DEFAULT_MIX_LISTENING_PORT, DEFAULT_NYM_NODE_HTTP_PORT, WG_PORT,
};
use nym_config::serde_helpers::de_maybe_stringified;
use nym_config::{
    must_get_home, parse_urls, read_config_from_toml_file, save_formatted_config_to_file,
    NymConfigTemplate, DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use tracing::{debug, error};
use url::Url;

mod entry_gateway;
mod exit_gateway;
mod mixnode;
pub mod persistence;
mod template;
mod upgrade_helpers;

const DEFAULT_NYMNODES_DIR: &str = "nym-nodes";

pub const DEFAULT_WIREGUARD_PORT: u16 = WG_PORT;
pub const DEFAULT_WIREGUARD_PREFIX: u8 = 16;
pub const DEFAULT_HTTP_PORT: u16 = DEFAULT_NYM_NODE_HTTP_PORT;
pub const DEFAULT_MIXNET_PORT: u16 = DEFAULT_MIX_LISTENING_PORT;

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

// a temporary solution until all "types" are run at the same time
#[derive(Debug, Default, Serialize, Deserialize, ValueEnum, Clone, Copy)]
pub enum NodeMode {
    #[default]
    Mixnode,
    EntryGateway,

    // those will be combined once some bugs are squashed
    ExitGatewayNR,
    ExitGatewayIPR,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    pub id: String,

    pub host: Host,

    #[serde(default)]
    pub http: Http,

    pub mixnet: Mixnet,

    #[serde(default)]
    pub wireguard: Wireguard,

    pub mode: NodeMode,

    pub storage_paths: NymNodePaths,

    pub mixnode: MixnodeConfig,

    pub entry_gateway: EntryGatewayConfig,

    pub exit_gateway: ExitGatewayConfig,

    #[serde(default)]
    pub logging: LoggingSettings,
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

    pub fn default_data_directory<P: AsRef<Path>>(config_path: P) -> Result<PathBuf, NymNodeError> {
        let config_path = config_path.as_ref();

        // the provided path does not point to .toml file
        if !config_path.is_file() {
            error!("'{}' does not point to a valid file", config_path.display());
            return Err(NymNodeError::DataDirDerivationFailure);
        }

        // we got a proper path to the .toml file
        let Some(config_dir) = config_path.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_path.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        let Some(config_dir_name) = config_dir.file_name() else {
            error!(
                "could not obtain parent directory name of '{}'. Have you used relative paths?",
                config_path.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        if config_dir_name != DEFAULT_CONFIG_DIR {
            error!(
                "the parent directory of '{}' ({}) is not {DEFAULT_CONFIG_DIR}. currently this is not supported",
                config_path.display(), config_dir_name.to_str().unwrap_or("UNKNOWN")
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        }

        let Some(node_dir) = config_dir.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_dir.display()
            );
            return Err(NymNodeError::DataDirDerivationFailure);
        };

        Ok(node_dir.join(DEFAULT_DATA_DIR))
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
    #[serde(deserialize_with = "de_maybe_stringified")]
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
    /// note: for legacy reasons, it defaults to port `8000` for standalone mixnodes.
    pub bind_address: SocketAddr,

    /// Path to assets directory of custom landing page of this node.
    #[serde(deserialize_with = "de_maybe_stringified")]
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
pub struct Mixnet {
    /// Address this node will bind to for listening for mixnet packets
    /// default: `0.0.0.0:1789`
    pub bind_address: SocketAddr,

    /// Addresses to nym APIs from which the node gets the view of the network.
    pub nym_api_urls: Vec<Url>,
}

impl Default for Mixnet {
    fn default() -> Self {
        // SAFETY:
        // our hardcoded values should always be valid
        #[allow(clippy::expect_used)]
        // is if there's anything set in the environment, otherwise fallback to mainnet
        let nym_api_urls = if let Ok(env_value) = env::var(var_names::NYM_API) {
            parse_urls(&env_value)
        } else {
            vec![mainnet::NYM_API.parse().expect("Invalid default API URL")]
        };

        Mixnet {
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DEFAULT_MIXNET_PORT),
            nym_api_urls,
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
