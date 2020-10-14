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
