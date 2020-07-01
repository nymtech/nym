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
pub struct ClientPathfinder {
    pub config_dir: PathBuf,
    pub private_mix_key: PathBuf,
    pub public_mix_key: PathBuf,

    identity_private_key: PathBuf,
    identity_public_key: PathBuf,
    encryption_private_key: PathBuf,
    encryption_public_key: PathBuf,
    shared_gateway_key: PathBuf,
}

impl ClientPathfinder {
    pub fn new(id: String) -> Self {
        let os_config_dir = dirs::config_dir().unwrap(); // grabs the OS default config dir
        let config_dir = os_config_dir.join("nym").join("clients").join(id);
        let private_mix_key = config_dir.join("private.pem");
        let public_mix_key = config_dir.join("public.pem");
        ClientPathfinder {
            config_dir,
            private_mix_key,
            public_mix_key,

            identity_private_key: Default::default(),
            identity_public_key: Default::default(),
            encryption_private_key: Default::default(),
            encryption_public_key: Default::default(),
            shared_gateway_key: Default::default(),
        }
    }

    pub fn new_from_config(config: &Config) -> Self {
        ClientPathfinder {
            config_dir: config.get_config_file_save_location(),
            private_mix_key: config.get_private_identity_key_file(),
            public_mix_key: config.get_public_identity_key_file(),

            identity_private_key: Default::default(),
            identity_public_key: Default::default(),
            encryption_private_key: Default::default(),
            encryption_public_key: Default::default(),
            shared_gateway_key: Default::default(),
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn private_identity_key(&self) -> &Path {
        &self.private_mix_key
    }

    pub fn public_identity_key(&self) -> &Path {
        &self.public_mix_key
    }
}
