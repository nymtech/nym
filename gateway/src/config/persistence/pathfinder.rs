// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
