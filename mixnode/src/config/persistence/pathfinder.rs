// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct MixNodePathfinder {
    identity_private_key: PathBuf,
    identity_public_key: PathBuf,
    private_sphinx_key: PathBuf,
    public_sphinx_key: PathBuf,
}

impl MixNodePathfinder {
    pub fn new_from_config(config: &Config) -> Self {
        MixNodePathfinder {
            identity_private_key: config.get_private_identity_key_file(),
            identity_public_key: config.get_public_identity_key_file(),
            private_sphinx_key: config.get_private_sphinx_key_file(),
            public_sphinx_key: config.get_public_sphinx_key_file(),
        }
    }

    pub fn private_identity_key(&self) -> &Path {
        &self.identity_private_key
    }

    pub fn public_identity_key(&self) -> &Path {
        &self.identity_public_key
    }

    pub fn private_encryption_key(&self) -> &Path {
        &self.private_sphinx_key
    }

    pub fn public_encryption_key(&self) -> &Path {
        &self.public_sphinx_key
    }
}
