// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{default_config_directory, default_data_directory};
use nym_config::serde_helpers::de_maybe_stringified;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME: &str = "private_identity.pem";
pub const DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME: &str = "public_identity.pem";
pub const DEFAULT_PRIVATE_SPHINX_KEY_FILENAME: &str = "private_sphinx.pem";
pub const DEFAULT_PUBLIC_SPHINX_KEY_FILENAME: &str = "public_sphinx.pem";

pub const DEFAULT_CLIENTS_STORAGE_FILENAME: &str = "db.sqlite";

pub const DEFAULT_NETWORK_REQUESTER_CONFIG_FILENAME: &str = "network_requester_config.toml";
pub const DEFAULT_NETWORK_REQUESTER_DATA_DIR: &str = "network-requester-data";

pub const DEFAULT_IP_PACKET_ROUTER_CONFIG_FILENAME: &str = "ip_packet_router_config.toml";
pub const DEFAULT_IP_PACKET_ROUTER_DATA_DIR: &str = "ip-packet-router-data";

pub const DEFAULT_WIREGUARD_CONFIG_FILENAME: &str = "wireguard.toml";
pub const DEFAULT_WIREGUARD_DATA_DIR: &str = "wireguard";

// pub const DEFAULT_DESCRIPTION_FILENAME: &str = "description.toml";

pub fn default_network_requester_data_dir<P: AsRef<Path>>(id: P) -> PathBuf {
    default_data_directory(id).join(DEFAULT_NETWORK_REQUESTER_DATA_DIR)
}

pub fn default_ip_packet_router_data_dir<P: AsRef<Path>>(id: P) -> PathBuf {
    default_data_directory(id).join(DEFAULT_IP_PACKET_ROUTER_DATA_DIR)
}

pub fn default_wireguard_data_dir<P: AsRef<Path>>(id: P) -> PathBuf {
    default_data_directory(id).join(DEFAULT_WIREGUARD_DATA_DIR)
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayPaths {
    pub keys: KeysPaths,

    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys and available client bandwidths.
    #[serde(alias = "persistent_storage")]
    pub clients_storage: PathBuf,

    /// Path to the configuration of the embedded network requester.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub network_requester_config: Option<PathBuf>,
    // pub node_description: PathBuf,

    // pub cosmos_bip39_mnemonic: PathBuf,
    /// Path to the configuration of the embedded ip packet router.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub ip_packet_router_config: Option<PathBuf>,

    /// Path to the configuration of the wireguard server.
    #[serde(deserialize_with = "de_maybe_stringified")]
    pub wireguard_config: Option<PathBuf>,
}

impl GatewayPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        GatewayPaths {
            keys: KeysPaths::new_default(id.as_ref()),
            clients_storage: default_data_directory(id).join(DEFAULT_CLIENTS_STORAGE_FILENAME),
            // node_description: default_config_filepath(id).join(DEFAULT_DESCRIPTION_FILENAME),
            network_requester_config: None,
            ip_packet_router_config: None,
            wireguard_config: None,
        }
    }

    pub fn new_empty() -> Self {
        GatewayPaths {
            keys: KeysPaths {
                private_identity_key_file: Default::default(),
                public_identity_key_file: Default::default(),
                private_sphinx_key_file: Default::default(),
                public_sphinx_key_file: Default::default(),
            },
            clients_storage: Default::default(),
            network_requester_config: None,
            ip_packet_router_config: None,
            wireguard_config: None,
        }
    }

    #[must_use]
    pub fn with_network_requester_config<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.network_requester_config = Some(path.as_ref().into());
        self
    }

    #[must_use]
    pub fn with_default_network_requester_config<P: AsRef<Path>>(self, id: P) -> Self {
        self.with_network_requester_config(
            default_config_directory(id).join(DEFAULT_NETWORK_REQUESTER_CONFIG_FILENAME),
        )
    }

    #[must_use]
    pub fn with_ip_packet_router_config<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.ip_packet_router_config = Some(path.as_ref().into());
        self
    }

    #[must_use]
    pub fn with_wireguard_config<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.wireguard_config = Some(path.as_ref().into());
        self
    }

    #[must_use]
    pub fn with_default_ip_packet_router_config<P: AsRef<Path>>(self, id: P) -> Self {
        self.with_ip_packet_router_config(
            default_config_directory(id).join(DEFAULT_IP_PACKET_ROUTER_CONFIG_FILENAME),
        )
    }

    #[must_use]
    pub fn with_default_wireguard_config<P: AsRef<Path>>(self, id: P) -> Self {
        self.with_wireguard_config(
            default_config_directory(id).join(DEFAULT_WIREGUARD_CONFIG_FILENAME),
        )
    }

    pub fn network_requester_config(&self) -> &Option<PathBuf> {
        &self.network_requester_config
    }

    pub fn ip_packet_router_config(&self) -> &Option<PathBuf> {
        &self.ip_packet_router_config
    }

    pub fn wireguard_config(&self) -> &Option<PathBuf> {
        &self.wireguard_config
    }

    pub fn private_identity_key(&self) -> &Path {
        self.keys.private_identity_key()
    }

    pub fn public_identity_key(&self) -> &Path {
        self.keys.public_identity_key()
    }

    pub fn private_encryption_key(&self) -> &Path {
        self.keys.private_encryption_key()
    }

    pub fn public_encryption_key(&self) -> &Path {
        self.keys.public_encryption_key()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct KeysPaths {
    /// Path to file containing private identity key.
    pub private_identity_key_file: PathBuf,

    /// Path to file containing public identity key.
    pub public_identity_key_file: PathBuf,

    /// Path to file containing private sphinx key.
    pub private_sphinx_key_file: PathBuf,

    /// Path to file containing public sphinx key.
    pub public_sphinx_key_file: PathBuf,
}

impl KeysPaths {
    pub fn new_default<P: AsRef<Path>>(id: P) -> Self {
        let data_dir = default_data_directory(id);

        KeysPaths {
            private_identity_key_file: data_dir.join(DEFAULT_PRIVATE_IDENTITY_KEY_FILENAME),
            public_identity_key_file: data_dir.join(DEFAULT_PUBLIC_IDENTITY_KEY_FILENAME),
            private_sphinx_key_file: data_dir.join(DEFAULT_PRIVATE_SPHINX_KEY_FILENAME),
            public_sphinx_key_file: data_dir.join(DEFAULT_PUBLIC_SPHINX_KEY_FILENAME),
        }
    }

    pub fn private_identity_key(&self) -> &Path {
        &self.private_identity_key_file
    }

    pub fn public_identity_key(&self) -> &Path {
        &self.public_identity_key_file
    }

    pub fn private_encryption_key(&self) -> &Path {
        &self.private_sphinx_key_file
    }

    pub fn public_encryption_key(&self) -> &Path {
        &self.public_sphinx_key_file
    }
}
