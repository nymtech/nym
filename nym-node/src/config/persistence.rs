// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::EntryGatewayError;
use nym_client_core_config_types::disk_persistence::{ClientKeysPaths, CommonClientPaths};
use nym_mixnode::config::persistence::paths::DEFAULT_DESCRIPTION_FILENAME;
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
pub const DEFAULT_X25519_PRIVATE_NOISE_KEY_FILENAME: &str = "x25519_noise";
pub const DEFAULT_X25519_PUBLIC_NOISE_KEY_FILENAME: &str = "x25519_noise.pub";
pub const DEFAULT_NYMNODE_DESCRIPTION_FILENAME: &str = "description.toml";

// Mixnode:

// Entry Gateway:
pub const DEFAULT_CLIENTS_STORAGE_FILENAME: &str = "clients.sqlite";
pub const DEFAULT_STATS_STORAGE_FILENAME: &str = "stats.sqlite";
pub const DEFAULT_MNEMONIC_FILENAME: &str = "cosmos_mnemonic";

// Exit Gateway:
pub const DEFAULT_ED25519_NR_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_nr_identity";
pub const DEFAULT_ED25519_NR_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_nr_identity.pub";
pub const DEFAULT_X25519_NR_PRIVATE_DH_KEY_FILENAME: &str = "x25519_nr_dh";
pub const DEFAULT_X25519_NR_PUBLIC_DH_KEY_FILENAME: &str = "x25519_nr_dh.pub";
pub const DEFAULT_NR_ACK_KEY_FILENAME: &str = "aes128ctr_nr_ack";
pub const DEFAULT_NR_REPLY_SURB_DB_FILENAME: &str = "nr_persistent_reply_store.sqlite";
pub const DEFAULT_NR_GATEWAYS_DB_FILENAME: &str = "nr_gateways_info_store.sqlite";

pub const DEFAULT_ED25519_IPR_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_ipr_identity";
pub const DEFAULT_ED25519_IPR_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_ipr_identity.pub";
pub const DEFAULT_X25519_IPR_PRIVATE_DH_KEY_FILENAME: &str = "x25519_ipr_dh";
pub const DEFAULT_X25519_IPR_PUBLIC_DH_KEY_FILENAME: &str = "x25519_ipr_dh.pub";
pub const DEFAULT_IPR_ACK_KEY_FILENAME: &str = "aes128ctr_ipr_ack";
pub const DEFAULT_IPR_REPLY_SURB_DB_FILENAME: &str = "ipr_persistent_reply_store.sqlite";
pub const DEFAULT_IPR_GATEWAYS_DB_FILENAME: &str = "ipr_gateways_info_store.sqlite";

pub const DEFAULT_ED25519_AUTH_PRIVATE_IDENTITY_KEY_FILENAME: &str = "ed25519_auth_identity";
pub const DEFAULT_ED25519_AUTH_PUBLIC_IDENTITY_KEY_FILENAME: &str = "ed25519_auth_identity.pub";
pub const DEFAULT_X25519_AUTH_PRIVATE_DH_KEY_FILENAME: &str = "x25519_auth_dh";
pub const DEFAULT_X25519_AUTH_PUBLIC_DH_KEY_FILENAME: &str = "x25519_auth_dh.pub";
pub const DEFAULT_AUTH_ACK_KEY_FILENAME: &str = "aes128ctr_auth_ack";
pub const DEFAULT_AUTH_REPLY_SURB_DB_FILENAME: &str = "auth_persistent_reply_store.sqlite";
pub const DEFAULT_AUTH_GATEWAYS_DB_FILENAME: &str = "auth_gateways_info_store.sqlite";

// Wireguard
pub const DEFAULT_X25519_WG_DH_KEY_FILENAME: &str = "x25519_wg_dh";
pub const DEFAULT_X25519_WG_PUBLIC_DH_KEY_FILENAME: &str = "x25519_wg_dh.pub";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NymNodePaths {
    pub keys: KeysPaths,

    /// Path to a file containing basic node description: human-readable name, website, details, etc.
    pub description: PathBuf,
}

impl NymNodePaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();

        NymNodePaths {
            keys: KeysPaths::new(data_dir),
            description: data_dir.join(DEFAULT_DESCRIPTION_FILENAME),
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

    /// Path to file containing x25519 noise private key.
    pub private_x25519_noise_key_file: PathBuf,

    /// Path to file containing x25519 noise public key.
    pub public_x25519_noise_key_file: PathBuf,
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
            private_x25519_noise_key_file: data_dir.join(DEFAULT_X25519_PRIVATE_NOISE_KEY_FILENAME),
            public_x25519_noise_key_file: data_dir.join(DEFAULT_X25519_PUBLIC_NOISE_KEY_FILENAME),
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

    pub fn x25519_noise_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_x25519_noise_key_file,
            &self.public_x25519_noise_key_file,
        )
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MixnodePaths {}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntryGatewayPaths {
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys, available client bandwidths and wireguard peers.
    pub clients_storage: PathBuf,

    /// Path to sqlite database containing all persistent stats data.
    pub stats_storage: PathBuf,

    /// Path to file containing cosmos account mnemonic used for zk-nym redemption.
    pub cosmos_mnemonic: PathBuf,

    pub authenticator: AuthenticatorPaths,
}

impl EntryGatewayPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        EntryGatewayPaths {
            clients_storage: data_dir.as_ref().join(DEFAULT_CLIENTS_STORAGE_FILENAME),
            stats_storage: data_dir.as_ref().join(DEFAULT_STATS_STORAGE_FILENAME),
            cosmos_mnemonic: data_dir.as_ref().join(DEFAULT_MNEMONIC_FILENAME),
            authenticator: AuthenticatorPaths::new(data_dir),
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
    /// Path to sqlite database containing all persistent data: messages for offline clients,
    /// derived shared keys, available client bandwidths and wireguard peers.
    pub clients_storage: PathBuf,

    /// Path to sqlite database containing all persistent stats data.
    pub stats_storage: PathBuf,

    pub network_requester: NetworkRequesterPaths,

    pub ip_packet_router: IpPacketRouterPaths,

    pub authenticator: AuthenticatorPaths,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkRequesterPaths {
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

    /// Normally this is a path to the file containing information about gateways used by this client,
    /// i.e. details such as their public keys, owner addresses or the network information.
    /// but in this case it just has the basic information of "we're using custom gateway".
    /// Due to how clients are started up, this file has to exist.
    pub gateway_registrations: PathBuf,
    // it's possible we might have to add credential storage here for return tickets
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
            gateway_registrations: data_dir.join(DEFAULT_NR_GATEWAYS_DB_FILENAME),
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
            gateway_registrations: self.gateway_registrations.clone(),

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

    /// Normally this is a path to the file containing information about gateways used by this client,
    /// i.e. details such as their public keys, owner addresses or the network information.
    /// but in this case it just has the basic information of "we're using custom gateway".
    /// Due to how clients are started up, this file has to exist.
    pub gateway_registrations: PathBuf,
    // it's possible we might have to add credential storage here for return tickets
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
            gateway_registrations: data_dir.join(DEFAULT_IPR_GATEWAYS_DB_FILENAME),
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
            gateway_registrations: self.gateway_registrations.clone(),

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
pub struct AuthenticatorPaths {
    /// Path to file containing authenticator ed25519 identity private key.
    pub private_ed25519_identity_key_file: PathBuf,

    /// Path to file containing authenticator ed25519 identity public key.
    pub public_ed25519_identity_key_file: PathBuf,

    /// Path to file containing authenticator x25519 diffie hellman private key.
    pub private_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing authenticator x25519 diffie hellman public key.
    pub public_x25519_diffie_hellman_key_file: PathBuf,

    /// Path to file containing key used for encrypting and decrypting the content of an
    /// acknowledgement so that nobody besides the client knows which packet it refers to.
    pub ack_key_file: PathBuf,

    /// Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
    pub reply_surb_database: PathBuf,

    /// Normally this is a path to the file containing information about gateways used by this client,
    /// i.e. details such as their public keys, owner addresses or the network information.
    /// but in this case it just has the basic information of "we're using custom gateway".
    /// Due to how clients are started up, this file has to exist.
    pub gateway_registrations: PathBuf,
    // it's possible we might have to add credential storage here for return tickets
}

impl AuthenticatorPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        AuthenticatorPaths {
            private_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_AUTH_PRIVATE_IDENTITY_KEY_FILENAME),
            public_ed25519_identity_key_file: data_dir
                .join(DEFAULT_ED25519_AUTH_PUBLIC_IDENTITY_KEY_FILENAME),
            private_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_AUTH_PRIVATE_DH_KEY_FILENAME),
            public_x25519_diffie_hellman_key_file: data_dir
                .join(DEFAULT_X25519_AUTH_PUBLIC_DH_KEY_FILENAME),
            ack_key_file: data_dir.join(DEFAULT_AUTH_ACK_KEY_FILENAME),
            reply_surb_database: data_dir.join(DEFAULT_AUTH_REPLY_SURB_DB_FILENAME),
            gateway_registrations: data_dir.join(DEFAULT_AUTH_GATEWAYS_DB_FILENAME),
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
            gateway_registrations: self.gateway_registrations.clone(),

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
            clients_storage: data_dir.join(DEFAULT_CLIENTS_STORAGE_FILENAME),
            stats_storage: data_dir.join(DEFAULT_STATS_STORAGE_FILENAME),
            network_requester: NetworkRequesterPaths::new(data_dir),
            ip_packet_router: IpPacketRouterPaths::new(data_dir),
            authenticator: AuthenticatorPaths::new(data_dir),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WireguardPaths {
    pub private_diffie_hellman_key_file: PathBuf,
    pub public_diffie_hellman_key_file: PathBuf,
}

impl WireguardPaths {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        let data_dir = data_dir.as_ref();
        WireguardPaths {
            private_diffie_hellman_key_file: data_dir.join(DEFAULT_X25519_WG_DH_KEY_FILENAME),
            public_diffie_hellman_key_file: data_dir.join(DEFAULT_X25519_WG_PUBLIC_DH_KEY_FILENAME),
        }
    }

    pub fn x25519_wireguard_storage_paths(&self) -> nym_pemstore::KeyPairPath {
        nym_pemstore::KeyPairPath::new(
            &self.private_diffie_hellman_key_file,
            &self.public_diffie_hellman_key_file,
        )
    }
}
