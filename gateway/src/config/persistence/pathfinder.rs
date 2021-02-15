// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct GatewayPathfinder {
    pub config_dir: PathBuf,
    pub private_sphinx_key: PathBuf,
    pub public_sphinx_key: PathBuf,
    pub private_identity_key: PathBuf,
    pub public_identity_key: PathBuf,
}

impl GatewayPathfinder {
    pub fn new_from_config(config: &Config) -> Self {
        GatewayPathfinder {
            config_dir: config.get_config_file_save_location(),
            private_sphinx_key: config.get_private_sphinx_key_file(),
            public_sphinx_key: config.get_public_sphinx_key_file(),
            private_identity_key: config.get_private_identity_key_file(),
            public_identity_key: config.get_public_identity_key_file(),
        }
    }

    pub fn private_identity_key(&self) -> &Path {
        &self.private_identity_key
    }

    pub fn public_identity_key(&self) -> &Path {
        &self.public_identity_key
    }

    pub fn private_encryption_key(&self) -> &Path {
        &self.private_sphinx_key
    }

    pub fn public_encryption_key(&self) -> &Path {
        &self.public_sphinx_key
    }
}
