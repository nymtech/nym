// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use nym_config::NymConfig;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ClientKeyPathfinder {
    pub identity_private_key: PathBuf,
    pub identity_public_key: PathBuf,
    pub encryption_private_key: PathBuf,
    pub encryption_public_key: PathBuf,
    pub gateway_shared_key: PathBuf,
    pub ack_key: PathBuf,
}

impl ClientKeyPathfinder {
    pub fn new(id: String) -> Self {
        let os_config_dir = dirs::config_dir().expect("no config directory known for this OS"); // grabs the OS default config dir
        let config_dir = os_config_dir.join("nym").join("clients").join(id);
        ClientKeyPathfinder {
            identity_private_key: config_dir.join("private_identity.pem"),
            identity_public_key: config_dir.join("public_identity.pem"),
            encryption_private_key: config_dir.join("private_encryption.pem"),
            encryption_public_key: config_dir.join("public_encryption.pem"),
            gateway_shared_key: config_dir.join("gateway_shared.pem"),
            ack_key: config_dir.join("ack_key.pem"),
        }
    }

    pub fn new_from_config<T: NymConfig>(config: &Config<T>) -> Self {
        ClientKeyPathfinder {
            identity_private_key: config.get_private_identity_key_file(),
            identity_public_key: config.get_public_identity_key_file(),
            encryption_private_key: config.get_private_encryption_key_file(),
            encryption_public_key: config.get_public_encryption_key_file(),
            gateway_shared_key: config.get_gateway_shared_key_file(),
            ack_key: config.get_ack_key_file(),
        }
    }

    pub fn any_file_exists(&self) -> bool {
        matches!(self.identity_public_key.try_exists(), Ok(true))
            || matches!(self.identity_private_key.try_exists(), Ok(true))
            || matches!(self.encryption_public_key.try_exists(), Ok(true))
            || matches!(self.encryption_private_key.try_exists(), Ok(true))
            || matches!(self.gateway_shared_key.try_exists(), Ok(true))
            || matches!(self.ack_key.try_exists(), Ok(true))
    }

    pub fn any_file_exists_and_return(&self) -> Option<PathBuf> {
        file_exists(&self.identity_public_key)
            .or_else(|| file_exists(&self.identity_private_key))
            .or_else(|| file_exists(&self.encryption_public_key))
            .or_else(|| file_exists(&self.encryption_private_key))
            .or_else(|| file_exists(&self.gateway_shared_key))
            .or_else(|| file_exists(&self.ack_key))
    }

    pub fn gateway_key_file_exists(&self) -> bool {
        matches!(self.gateway_shared_key.try_exists(), Ok(true))
    }

    pub fn private_identity_key(&self) -> &Path {
        &self.identity_private_key
    }

    pub fn public_identity_key(&self) -> &Path {
        &self.identity_public_key
    }

    pub fn private_encryption_key(&self) -> &Path {
        &self.encryption_private_key
    }

    pub fn public_encryption_key(&self) -> &Path {
        &self.encryption_public_key
    }

    pub fn gateway_shared_key(&self) -> &Path {
        &self.gateway_shared_key
    }

    pub fn ack_key(&self) -> &Path {
        &self.ack_key
    }
}

fn file_exists(path: &Path) -> Option<PathBuf> {
    if matches!(path.try_exists(), Ok(true)) {
        return Some(path.to_path_buf());
    }
    None
}
