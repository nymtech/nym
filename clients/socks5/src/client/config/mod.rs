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

use crate::client::config::template::config_template;
use client_core::config::Config as BaseConfig;
pub use client_core::config::MISSING_VALUE;
use config::NymConfig;
use nymsphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod template;

const DEFAULT_LISTENING_PORT: u16 = 1080;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    base: BaseConfig<Config>,

    socks5: Socks5,
}

impl NymConfig for Config {
    fn template() -> &'static str {
        config_template()
    }

    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("socks5-clients")
    }

    fn root_directory(&self) -> PathBuf {
        self.base.get_nym_root_directory()
    }

    fn config_directory(&self) -> PathBuf {
        self.root_directory()
            .join(self.base.get_id())
            .join("config")
    }

    fn data_directory(&self) -> PathBuf {
        self.root_directory().join(self.base.get_id()).join("data")
    }
}

impl Config {
    pub fn new<S: Into<String>>(id: S, provider_mix_address: S) -> Self {
        Config {
            base: BaseConfig::new(id),
            socks5: Socks5::new(provider_mix_address),
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.socks5.listening_port = port;
        self
    }

    pub fn with_provider_mix_address(mut self, address: String) -> Self {
        self.socks5.provider_mix_address = address;
        self
    }

    // getters
    pub fn get_config_file_save_location(&self) -> PathBuf {
        self.config_directory().join(Self::config_file_name())
    }

    pub fn get_provider_mix_address(&self) -> Recipient {
        Recipient::try_from_base58_string(&self.socks5.provider_mix_address)
            .expect("malformed provider address")
    }

    pub fn get_base(&self) -> &BaseConfig<Self> {
        &self.base
    }

    pub fn get_base_mut(&mut self) -> &mut BaseConfig<Self> {
        &mut self.base
    }

    pub fn get_listening_port(&self) -> u16 {
        self.socks5.listening_port
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socks5 {
    /// The port on which the client will be listening for incoming requests
    listening_port: u16,

    /// The mix address of the provider to which all requests are going to be sent.
    provider_mix_address: String,
}

impl Socks5 {
    pub fn new<S: Into<String>>(provider_mix_address: S) -> Self {
        Socks5 {
            listening_port: DEFAULT_LISTENING_PORT,
            provider_mix_address: provider_mix_address.into(),
        }
    }
}

impl Default for Socks5 {
    fn default() -> Self {
        Socks5 {
            listening_port: DEFAULT_LISTENING_PORT,
            provider_mix_address: "".into(),
        }
    }
}

#[cfg(test)]
mod client_config {
    use super::*;

    #[test]
    fn after_saving_default_config_the_loaded_one_is_identical() {
        // need to figure out how to do something similar but without touching the disk
        // or the file system at all...
        let temp_location = tempfile::tempdir().unwrap().path().join("config.toml");
        let fake_address = "CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@FioFa8nMmPpQnYi7JyojoTuwGLeyNS8BF4ChPr29zUML";
        let default_config = Config::new("foomp", fake_address);
        default_config
            .save_to_file(Some(temp_location.clone()))
            .unwrap();

        let loaded_config = Config::load_from_file(Some(temp_location), None).unwrap();

        assert_eq!(default_config, loaded_config);
    }
}
