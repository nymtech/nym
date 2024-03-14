// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// Global:
pub const DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_identity";
pub const DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_identity.pub";
pub const DEFAULT_X25519_PRIVATE_SPHINX_KEY_FILENAME: &str = "x25519_sphinx";
pub const DEFAULT_X25519_PUBLIC_SPHINX_KEY_FILENAME: &str = "x25519_sphinx.pub";

// Mixnode:
pub const DEFAULT_DESCRIPTION_FILENAME: &str = "description.toml";

// Entry Gateway:
pub const DEFAULT_CLIENTS_STORAGE_FILENAME: &str = "clients.sqlite";

// Exit Gateway:
pub const DEFAULT_NETWORK_REQUESTER_CONFIG_FILENAME: &str = "network_requester_config.toml";
pub const DEFAULT_IP_PACKET_ROUTER_CONFIG_FILENAME: &str = "ip_packet_router_config.toml";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NymNodePaths {
    pub keys: KeysPaths,
}

impl NymNodePaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();

        NymNodePaths {
            keys: KeysPaths::new(data_dir),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeysPaths {
    /// Path to file containing ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing x25519 sphinx private key.
    pub private_x25519_sphinx_key_file: PathBuf,

    /// Path to file containing x25519 sphinx public key.
    pub public_x25519_sphinx_key_file: PathBuf,
}

impl KeysPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();

        KeysPaths {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_sphinx_key_file: data_dir
                .join(DEFAULT_X25519_PRIVATE_SPHINX_KEY_FILENAME),
            public_x25519_sphinx_key_file: data_dir.join(DEFAULT_X25519_PUBLIC_SPHINX_KEY_FILENAME),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.public_ed25519_identity_key_file,
            &self.private_ed25519_identity_key_file,
        )
    }

    pub fn x25519_sphinx_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.public_x25519_sphinx_key_file,
            &self.private_x25519_sphinx_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixnodePaths {
    /// Path to a file containing basic node description: human-readable name, description, link and location.
    // Artifact of a bygone era. For now leave it here for easier mixnode compatibility;
    // To be replaced by just putting this information as part of the self-described API.
    pub node_description: PathBuf,
}

impl MixnodePaths {
    pub fn new<P: AsRef<Path>>(config_dir: P) -> Self {
        MixnodePaths {
            node_description: config_dir.as_ref().join(DEFAULT_DESCRIPTION_FILENAME),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayPaths {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys and available client bandwidths.
    pub clients_storage: PathBuf,
}

impl EntryGatewayPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        EntryGatewayPaths {
            clients_storage: data_dir.as_ref().join(DEFAULT_CLIENTS_STORAGE_FILENAME),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayPaths {
    /// Path to the configuration of the embedded network requester.
    pub network_requester_config: PathBuf,

    /// Path to the configuration of the embedded ip packet router.
    pub ip_packet_router_config: PathBuf,
}

impl ExitGatewayPaths {
    pub fn new<P: AsRef<Path>>(config_dir: P) -> Self {
        let config_dir = config_dir.as_ref();
        ExitGatewayPaths {
            network_requester_config: config_dir.join(DEFAULT_NETWORK_REQUESTER_CONFIG_FILENAME),
            ip_packet_router_config: config_dir.join(DEFAULT_IP_PACKET_ROUTER_CONFIG_FILENAME),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardPaths {
    // pub keys:
}

impl WireguardPaths {
    pub fn new<P: AsRef<Path>>(_data_dir: P) -> Self {
        WireguardPaths {}
    }
}
