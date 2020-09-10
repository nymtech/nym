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
use config::NymConfig;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ClientKeyPathfinder {
    identity_private_key: PathBuf,
    identity_public_key: PathBuf,
    encryption_private_key: PathBuf,
    encryption_public_key: PathBuf,
    gateway_shared_key: PathBuf,
    ack_key: PathBuf,
}

impl ClientKeyPathfinder {
    pub fn new(id: String) -> Self {
        let os_config_dir = dirs::config_dir().expect("no config directory known for this OS"); // grabs the OS default config dir
        let config_dir = os_config_dir.join("nym").join("clients").join(id);
        ClientKeyPathfinder {
            identity_private_key: config_dir.join("private_identity.pem"),
            identity_public_key: config_dir.join("public_identity.pem"),
            encryption_private_key: config_dir.join("public_encryption.pem"),
            encryption_public_key: config_dir.join("private_encryption.pem"),
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
