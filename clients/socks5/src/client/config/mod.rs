// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::config::template::config_template;
use client_core::config::Config as BaseConfig;
pub use client_core::config::MISSING_VALUE;
use config::defaults::DEFAULT_SOCKS5_LISTENING_PORT;
use config::NymConfig;
use nymsphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod template;

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

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
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
            listening_port: DEFAULT_SOCKS5_LISTENING_PORT,
            provider_mix_address: provider_mix_address.into(),
        }
    }
}

impl Default for Socks5 {
    fn default() -> Self {
        Socks5 {
            listening_port: DEFAULT_SOCKS5_LISTENING_PORT,
            provider_mix_address: "".into(),
        }
    }
}
