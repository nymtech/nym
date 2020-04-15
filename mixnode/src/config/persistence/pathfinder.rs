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
use pemstore::pathfinder::PathFinder;
use std::path::PathBuf;

#[derive(Debug)]
pub struct MixNodePathfinder {
    pub config_dir: PathBuf,
    pub private_sphinx_key: PathBuf,
    pub public_sphinx_key: PathBuf,
}

impl MixNodePathfinder {
    pub fn new_from_config(config: &Config) -> Self {
        MixNodePathfinder {
            config_dir: config.get_config_file_save_location(),
            private_sphinx_key: config.get_private_sphinx_key_file(),
            public_sphinx_key: config.get_public_sphinx_key_file(),
        }
    }
}

impl PathFinder for MixNodePathfinder {
    fn config_dir(&self) -> PathBuf {
        self.config_dir.clone()
    }

    fn private_identity_key(&self) -> PathBuf {
        // TEMPORARILY USE SAME KEYS AS ENCRYPTION
        self.private_sphinx_key.clone()
    }

    fn public_identity_key(&self) -> PathBuf {
        // TEMPORARILY USE SAME KEYS AS ENCRYPTION
        self.public_sphinx_key.clone()
    }

    fn private_encryption_key(&self) -> Option<PathBuf> {
        Some(self.private_sphinx_key.clone())
    }

    fn public_encryption_key(&self) -> Option<PathBuf> {
        Some(self.public_sphinx_key.clone())
    }
}
