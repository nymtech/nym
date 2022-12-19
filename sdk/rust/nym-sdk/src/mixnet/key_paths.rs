use client_core::{
    client::key_manager::KeyManager, config::persistence::key_pathfinder::ClientKeyPathfinder,
};
use crypto::asymmetric::{encryption, identity};
use gateway_requests::registration::handshake::SharedKeys;
use nymsphinx::acknowledgements::AckKey;

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum KeyMode {
    /// Use existing key files if they exists, otherwise create new ones.
    Keep,
    /// Create new keys, overwriting any potential previously existing keys.
    Overwrite,
}

impl KeyMode {
    pub(crate) fn is_keep(&self) -> bool {
        matches!(self, KeyMode::Keep)
    }
}

pub enum GatewayKeyMode {
    /// Keep shared gateway key if found, otherwise create a new one.
    Keep,
    /// Create a new shared key and overwrite any potential existing one.
    Overwrite,
}

impl GatewayKeyMode {
    pub(crate) fn is_keep(&self) -> bool {
        matches!(self, GatewayKeyMode::Keep)
    }
}

#[derive(Clone, Debug)]
pub struct KeyPaths {
    // Determines how to handle existing key files found.
    pub operating_mode: KeyMode,

    // Client identity keys
    pub private_identity: PathBuf,
    pub public_identity: PathBuf,

    // Client encryption keys
    pub private_encryption: PathBuf,
    pub public_encryption: PathBuf,

    // Key for handling acks
    pub ack_key: PathBuf,

    // Key setup after authenticating with a gateway
    pub gateway_shared_key: PathBuf,

    // The key isn't much use without knowing which entity it refers to.
    // This is an `Option` in case the end user might want to read write keys to storage, but
    // handle the gateway configuration manually through `set_gateway_endpoint`.
    // WIP(JON): make it an option?
    pub gateway_endpoint_config: PathBuf,

    pub credential_database_path: PathBuf,

    pub reply_surb_database_path: PathBuf,
}

pub struct Keys {
    pub identity_keypair: identity::KeyPair,
    pub encryption_keypair: encryption::KeyPair,
    pub ack_key: AckKey,
    pub gateway_shared_key: SharedKeys,
}

//#[derive(Clone, Debug)]
//pub struct GatewaySetup {
//    pub gateway_shared_key: PathBuf,
//    pub gateway_endpoint_config: GatewayEndpointConfig,
//    //pub gateway_endpoint_config: GatewayConfig,
//}

//#[derive(Clone, Debug)]
//pub enum GatewayConfig {
//    File(PathBuf),
//    Struct(GatewayEndpointConfig),
//}

impl KeyPaths {
    pub fn new_from_dir(operating_mode: KeyMode, dir: &Path) -> Self {
        assert!(!dir.is_file(), "WIP");
        Self {
            // These filenames were chosen to match the ones we use in `nym-client`. Consider
            // changing the defaults
            operating_mode,
            private_identity: dir.join("private_identity.pem"),
            public_identity: dir.join("public_identity.pem"),
            private_encryption: dir.join("private_encryption.pem"),
            public_encryption: dir.join("public_encryption.pem"),
            ack_key: dir.join("ack_key.pem"),
            gateway_shared_key: dir.join("gateway_shared.pem"),
            gateway_endpoint_config: dir.join("gateway_endpoint_config.toml"),
            credential_database_path: dir.join("db.sqlite"),
            reply_surb_database_path: dir.join("persistent_reply_store.sqlite"),
        }
    }
}

impl From<KeyPaths> for ClientKeyPathfinder {
    fn from(paths: KeyPaths) -> Self {
        Self {
            identity_private_key: paths.private_identity,
            identity_public_key: paths.public_identity,
            encryption_private_key: paths.private_encryption,
            encryption_public_key: paths.public_encryption,
            gateway_shared_key: paths.gateway_shared_key,
            ack_key: paths.ack_key,
        }
    }
}

impl From<Keys> for KeyManager {
    fn from(keys: Keys) -> Self {
        KeyManager::new_from_keys(
            keys.identity_keypair,
            keys.encryption_keypair,
            keys.gateway_shared_key,
            keys.ack_key,
        )
    }
}

//pub struct GatewayConfigPath {
//    pub path: PathBuf,
//}
//
//impl GatewayConfigPath {
//    pub fn new_from_dir(dir: PathBuf) -> Self {
//        assert!(!dir.is_file(), "WIP");
//        Self {
//            path: dir.join("gateway_endpoint_config.toml"),
//        }
//    }
//}
