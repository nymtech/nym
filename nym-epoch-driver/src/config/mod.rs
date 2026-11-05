// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::ConfigOverridableArgs;
use crate::config::template::CONFIG_TEMPLATE;
use anyhow::bail;
use nym_config::{
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, NYM_DIR, NymConfigTemplate, must_get_home,
    read_config_from_toml_file, save_formatted_config_to_file,
};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

mod template;

const DEFAULT_EPOCH_DRIVER_DIR: &str = "nym-epoch-driver";

/// Get default path to the driver's config file.
/// It should get resolved to `$HOME/.nym/nym-epoch-driver/config/config.toml`
pub fn default_config_filepath() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_EPOCH_DRIVER_DIR)
        .join(DEFAULT_CONFIG_DIR)
        .join(DEFAULT_CONFIG_FILENAME)
}

#[derive(Debug, Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Config {
    /// Url to the nyxd instance the driver queries and sends its transactions through.
    #[zeroize(skip)]
    pub nyxd_url: Url,

    /// Mnemonic of the mixnet contract's rewarding validator account.
    pub mnemonic: bip39::Mnemonic,

    #[zeroize(skip)]
    pub http: Http,

    pub metrics: Metrics,

    #[zeroize(skip)]
    pub debug: Debug,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new(nyxd_url: Url, mnemonic: bip39::Mnemonic, metrics_bearer_token: String) -> Self {
        Config {
            nyxd_url,
            mnemonic,
            http: Http {
                bind_address: Http::DEFAULT_BIND_ADDRESS,
            },
            metrics: Metrics {
                bearer_token: metrics_bearer_token,
            },
            debug: Debug::default(),
        }
    }

    pub fn with_overrides(mut self, overrides: ConfigOverridableArgs) -> Self {
        if let Some(nyxd_url) = overrides.nyxd_url {
            self.nyxd_url = nyxd_url;
        }
        if let Some(bind_address) = overrides.http_bind_address {
            self.http.bind_address = bind_address;
        }
        if let Some(settle_lag) = overrides.settle_lag {
            self.debug.settle_lag = settle_lag;
        }
        self
    }

    /// The metrics listener must never be exposed without authentication.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.metrics.bearer_token.is_empty() {
            bail!("the metrics bearer token must not be empty")
        }
        Ok(())
    }

    pub fn read_from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        read_config_from_toml_file(path)
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        save_formatted_config_to_file(self, path)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Http {
    /// Socket address the http api binds to.
    pub bind_address: SocketAddr,
}

impl Http {
    pub const DEFAULT_BIND_ADDRESS: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
}

#[derive(Debug, Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Metrics {
    /// Bearer token every request to the metrics route must carry.
    pub bearer_token: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Debug {
    /// Number of blocks a cadence boundary must lie below the tip before the network view is
    /// produced at it.
    pub settle_lag: u64,
}

impl Debug {
    pub const DEFAULT_SETTLE_LAG: u64 = 10;
}

impl Default for Debug {
    fn default() -> Self {
        Debug {
            settle_lag: Self::DEFAULT_SETTLE_LAG,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_token(metrics_bearer_token: &str) -> Config {
        Config::new(
            "https://rpc.nymtech.net".parse().unwrap(),
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
                .parse()
                .unwrap(),
            metrics_bearer_token.to_string(),
        )
    }

    #[test]
    fn only_an_empty_metrics_bearer_token_is_refused() {
        assert!(config_with_token("").validate().is_err());
        assert!(config_with_token("token").validate().is_ok());
    }
}
