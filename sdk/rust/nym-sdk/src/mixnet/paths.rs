use nym_client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

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

/// Set of storage paths that the client will use if it is setup to persist keys, credentials, and
/// reply-SURBs.
#[derive(Clone, Debug)]
pub struct StoragePaths {
    /// Determines how to handle existing key files found.
    pub operating_mode: KeyMode,

    /// Client private identity key
    pub private_identity: PathBuf,
    /// Client public identity key
    pub public_identity: PathBuf,

    /// Client private encryption key
    pub private_encryption: PathBuf,
    /// Client public encryption key
    pub public_encryption: PathBuf,

    /// Key for handling acks
    pub ack_key: PathBuf,

    /// Key setup after authenticating with a gateway
    pub gateway_shared_key: PathBuf,

    /// The key isn't much use without knowing which entity it refers to.
    pub gateway_endpoint_config: PathBuf,

    /// The database containing credentials
    pub credential_database_path: PathBuf,

    /// The database storing reply surbs in-between sessions
    pub reply_surb_database_path: PathBuf,
}

impl StoragePaths {
    /// Create a set of storage paths from a given directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if it is passed a path to an existing file instead of a
    /// directory.
    pub fn new_from_dir(operating_mode: KeyMode, dir: &Path) -> Result<Self> {
        if dir.is_file() {
            return Err(Error::ExpectedDirectory(dir.to_owned()));
        }

        Ok(Self {
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
        })
    }
}

impl From<StoragePaths> for ClientKeyPathfinder {
    fn from(paths: StoragePaths) -> Self {
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

impl<T> From<&nym_client_core::config::Config<T>> for StoragePaths {
    fn from(value: &nym_client_core::config::Config<T>) -> Self {
        Self {
            operating_mode: KeyMode::Keep,
            private_identity: value.get_private_identity_key_file(),
            public_identity: value.get_public_identity_key_file(),
            private_encryption: value.get_private_encryption_key_file(),
            public_encryption: value.get_public_encryption_key_file(),
            ack_key: value.get_ack_key_file(),
            gateway_shared_key: value.get_gateway_shared_key_file(),
            gateway_endpoint_config: Default::default(),
            credential_database_path: value.get_database_path(),
            reply_surb_database_path: value.get_reply_surb_database_path(),
        }
    }
}
