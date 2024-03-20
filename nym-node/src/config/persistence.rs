// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::EntryGatewayError;
use nym_client_core_config_types::disk_persistence::{ClientKeysPaths, CommonClientPaths};
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tracing::info;
use zeroize::Zeroizing;

// Global:
pub const DEFAULT_ED25519_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_identity";
pub const DEFAULT_ED25519_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_identity.pub";
pub const DEFAULT_X25519_PRIVATE_SPHINX_KEY_FILENAME: &str = "x25519_sphinx";
pub const DEFAULT_X25519_PUBLIC_SPHINX_KEY_FILENAME: &str = "x25519_sphinx.pub";

// Mixnode:
pub const DEFAULT_DESCRIPTION_FILENAME: &str = "description.toml";

// Entry Gateway:
pub const DEFAULT_CLIENTS_STORAGE_FILENAME: &str = "clients.sqlite";
pub const DEFAULT_MNEMONIC_FILENAME: &str = "cosmos_mnemonic";

// Exit Gateway:
pub const DEFAULT_ED25519_NR_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_nr_identity";
pub const DEFAULT_ED25519_NR_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_nr_identity.pub";
pub const DEFAULT_X25519_NR_PRIVATE_DH_KEY_FILENAME: &str = "x25519_nr_dh";
pub const DEFAULT_X25519_NR_PUBLIC_DH_KEY_FILENAME: &str = "x25519_nr_dh.pub";
pub const DEFAULT_NR_ACK_KEY_FILENAME: &str = "aes128ctr_nr_ack";
pub const DEFAULT_NR_REPLY_SURB_DB_FILENAME: &str = "nr_persistent_reply_store.sqlite";

pub const DEFAULT_ED25519_IPR_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_ipr_identity";
pub const DEFAULT_ED25519_IPR_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_ipr_identity.pub";
pub const DEFAULT_X25519_IPR_PRIVATE_DH_KEY_FILENAME: &str = "x25519_ipr_dh";
pub const DEFAULT_X25519_IPR_PUBLIC_DH_KEY_FILENAME: &str = "x25519_ipr_dh.pub";
pub const DEFAULT_IPR_ACK_KEY_FILENAME: &str = "aes128ctr_ipr_ack";
pub const DEFAULT_IPR_REPLY_SURB_DB_FILENAME: &str = "ipr_persistent_reply_store.sqlite";

// pub const DEFAULT_NETWORK_REQUESTER_CONFIG_FILENAME: &str = "network_requester_config.toml";
// pub const DEFAULT_IP_PACKET_ROUTER_CONFIG_FILENAME: &str = "ip_packet_router_config.toml";

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
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        )
    }

    pub fn x25519_sphinx_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_sphinx_key_file,
            &self.public_x25519_sphinx_key_file,
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

    /// Path to file containing cosmos account mnemonic used for zk-nym redemption.
    pub cosmos_mnemonic: PathBuf,
}

impl EntryGatewayPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        EntryGatewayPaths {
            clients_storage: data_dir.as_ref().join(DEFAULT_CLIENTS_STORAGE_FILENAME),
            cosmos_mnemonic: data_dir.as_ref().join(DEFAULT_MNEMONIC_FILENAME),
        }
    }

    pub fn load_mnemonic_from_file(&self) -> Result<Zeroizing<bip39::Mnemonic>, EntryGatewayError> {
        let stringified =
            Zeroizing::new(fs::read_to_string(&self.cosmos_mnemonic).map_err(|source| {
                EntryGatewayError::MnemonicLoadFailure {
                    path: self.cosmos_mnemonic.clone(),
                    source,
                }
            })?);

        Ok(Zeroizing::new(bip39::Mnemonic::parse::<&str>(
            stringified.as_ref(),
        )?))
    }

    pub fn save_mnemonic_to_file(
        &self,
        mnemonic: &bip39::Mnemonic,
    ) -> Result<(), EntryGatewayError> {
        // wrapper for io errors
        fn _save_to_file(path: &Path, mnemonic: &bip39::Mnemonic) -> io::Result<()> {
            if let Some(parent) = path.parent() {
                create_dir_all(parent)?;
            }
            info!("saving entry gateway mnemonic to '{}'", path.display());

            let stringified = Zeroizing::new(mnemonic.to_string());
            fs::write(path, &stringified)
        }

        _save_to_file(&self.cosmos_mnemonic, mnemonic).map_err(|source| {
            EntryGatewayError::MnemonicSaveFailure {
                path: self.cosmos_mnemonic.clone(),
                source,
            }
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExitGatewayPaths {
    pub network_requester: NetworkRequesterPaths,

    pub ip_packet_router: IpPacketRouterPaths,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkRequesterPaths {
    // NR:
    /// Path to file containing network requester ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing network requester ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing network requester x25519 diffie hellman private key.
    pub private_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing network requester x25519 diffie hellman public key.
    pub public_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,
    // GW: only ephemeral
}

impl NetworkRequesterPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        NetworkRequesterPaths {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_NR_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_NR_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_NR_PRIVATE_DH_KEY_FILENAME),
            public_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_NR_PUBLIC_DH_KEY_FILENAME),
            ack_key_file: data_dir.join(DEFAULT_NR_ACK_KEY_FILENAME),
            reply_surb_database: data_dir.join(DEFAULT_NR_REPLY_SURB_DB_FILENAME),
        }
    }

    pub fn to_common_client_paths(&self) -> CommonClientPaths {
        CommonClientPaths {
            keys: ClientKeysPaths {
                private_identity_key_file: self.private_ed25519_identity_key_file.clone(),
                public_identity_key_file: self.public_ed25519_identity_key_file.clone(),
                private_encryption_key_file: self.private_x25519_diffie_hellman_key_file.clone(),
                public_encryption_key_file: self.public_x25519_diffie_hellman_key_file.clone(),
                ack_key_file: self.ack_key_file.clone(),
            },
            // should be able to get away without it
            gateway_registrations: Default::default(),

            // not needed for embedded providers
            credentials_database: Default::default(),
            reply_surb_database: self.reply_surb_database.clone(),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        )
    }

    pub fn x25519_diffie_hellman_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_diffie_hellman_key_file,
            &self.public_x25519_diffie_hellman_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IpPacketRouterPaths {
    // IPR:
    /// Path to file containing ip packet router ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ip packet router ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing ip packet router x25519 diffie hellman private key.
    pub private_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing ip packet router x25519 diffie hellman public key.
    pub public_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,
    // GW: only ephemeral
}

impl IpPacketRouterPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        IpPacketRouterPaths {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_IPR_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_IPR_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_IPR_PRIVATE_DH_KEY_FILENAME),
            public_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_IPR_PUBLIC_DH_KEY_FILENAME),
            ack_key_file: data_dir.join(DEFAULT_IPR_ACK_KEY_FILENAME),
            reply_surb_database: data_dir.join(DEFAULT_IPR_REPLY_SURB_DB_FILENAME),
        }
    }

    pub fn to_common_client_paths(&self) -> CommonClientPaths {
        CommonClientPaths {
            keys: ClientKeysPaths {
                private_identity_key_file: self.private_ed25519_identity_key_file.clone(),
                public_identity_key_file: self.public_ed25519_identity_key_file.clone(),
                private_encryption_key_file: self.private_x25519_diffie_hellman_key_file.clone(),
                public_encryption_key_file: self.public_x25519_diffie_hellman_key_file.clone(),
                ack_key_file: self.ack_key_file.clone(),
            },
            // should be able to get away without it
            gateway_registrations: Default::default(),

            // not needed for embedded providers
            credentials_database: Default::default(),
            reply_surb_database: self.reply_surb_database.clone(),
        }
    }

    pub fn ed25519_identity_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_ed25519_identity_key_file,
            &self.public_ed25519_identity_key_file,
        )
    }

    pub fn x25519_diffie_hellman_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_diffie_hellman_key_file,
            &self.public_x25519_diffie_hellman_key_file,
        )
    }
}

impl ExitGatewayPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        ExitGatewayPaths {
            network_requester: NetworkRequesterPaths::new(data_dir),
            ip_packet_router: IpPacketRouterPaths::new(data_dir),
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
