// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::paths::ValidatorRewarderPaths;
use crate::config::r#override::ConfigOverride;
use crate::config::template::CONFIG_TEMPLATE;
use crate::error::NymRewarderError;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd;
use nym_validator_client::nyxd::Coin;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::debug;
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub mod r#override;
pub mod persistence;
mod template;

const DEFAULT_REWARDER_DIR: &str = "validators-rewarder";

#[allow(clippy::inconsistent_digit_grouping)]
const DEFAULT_MIX_REWARDING_BUDGET: u128 = 1000_000000;
const DEFAULT_MIX_REWARDING_DENOM: &str = "unym";

const DEFAULT_EPOCH_DURATION: Duration = Duration::from_secs(60 * 60);

/// Get default path to rewarder's config directory.
/// It should get resolved to `$HOME/.nym/validators-rewarder/config`
pub fn default_config_directory() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_REWARDER_DIR)
        .join(DEFAULT_CONFIG_DIR)
}

/// Get default path to rewarder's config file.
/// It should get resolved to `$HOME/.nym/validators-rewarder/config/config.toml`
pub fn default_config_filepath() -> PathBuf {
    default_config_directory().join(DEFAULT_CONFIG_FILENAME)
}

/// Get default path to rewarder's data directory where files, such as keys, are stored.
/// It should get resolved to `$HOME/.nym/validators-rewarder/data`
pub fn default_data_directory() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_REWARDER_DIR)
        .join(DEFAULT_DATA_DIR)
}

#[derive(Debug, Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    #[zeroize(skip)]
    pub(crate) save_path: Option<PathBuf>,

    #[zeroize(skip)]
    pub rewarding: Rewarding,

    #[zeroize(skip)]
    pub nyxd_scraper: NyxdScraper,

    #[serde(flatten)]
    pub base: Base,

    #[zeroize(skip)]
    pub storage_paths: ValidatorRewarderPaths,
}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    pub fn new(mnemonic: bip39::Mnemonic) -> Self {
        let network = NymNetworkDetails::new_from_env();

        Config {
            save_path: None,
            rewarding: Rewarding::default(),
            nyxd_scraper: NyxdScraper {
                websocket_url: network.endpoints[0]
                    .websocket_url()
                    .expect("TODO: hardcoded websocket url is not available"),
            },
            base: Base {
                upstream_nyxd: network.endpoints[0].nyxd_url(),
                mnemonic,
            },
            storage_paths: Default::default(),
        }
    }

    pub fn scraper_config(&self) -> nyxd_scraper::Config {
        nyxd_scraper::Config {
            websocket_url: self.nyxd_scraper.websocket_url.clone(),
            rpc_url: self.base.upstream_nyxd.clone(),
            database_path: self.storage_paths.nyxd_scraper.clone(),
        }
    }

    pub fn rpc_client_config(&self) -> nyxd::Config {
        // TEMP
        nyxd::Config::try_from_nym_network_details(&NymNetworkDetails::new_from_env())
            .expect("failed to create nyxd client config")
    }

    pub fn ensure_is_valid(&self) -> Result<(), NymRewarderError> {
        self.rewarding.ratios.ensure_is_valid()?;
        Ok(())
    }

    pub fn r#override<O: ConfigOverride>(&mut self, r#override: O) {
        r#override.override_config(self)
    }

    pub fn with_override<O: ConfigOverride>(mut self, r#override: O) -> Self {
        self.r#override(r#override);
        self
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, NymRewarderError> {
        let path = path.as_ref();
        let mut loaded: Config = read_config_from_toml_file(path).map_err(|source| {
            NymRewarderError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            }
        })?;
        loaded.ensure_is_valid()?;
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    pub fn read_from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, NymRewarderError> {
        Self::read_from_path(path)
    }

    pub fn default_location() -> PathBuf {
        default_config_filepath()
    }

    pub fn save_to_default_location(&self) -> io::Result<()> {
        let config_save_location: PathBuf = Self::default_location();
        save_formatted_config_to_file(self, config_save_location)
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        save_formatted_config_to_file(self, path)
    }
}

#[derive(Debug, Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Base {
    /// Url to the upstream instance of nyxd to use for any queries.
    #[zeroize(skip)]
    pub upstream_nyxd: Url,

    /// Mnemonic to the nyx account distributing the rewards
    pub(crate) mnemonic: bip39::Mnemonic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rewarding {
    ///
    pub epoch_budget: Coin,

    #[serde(with = "humantime_serde")]
    pub epoch_duration: Duration,

    pub ratios: RewardingRatios,
}

impl Default for Rewarding {
    fn default() -> Self {
        Rewarding {
            epoch_budget: Coin::new(DEFAULT_MIX_REWARDING_BUDGET, DEFAULT_MIX_REWARDING_DENOM),
            epoch_duration: DEFAULT_EPOCH_DURATION,
            ratios: RewardingRatios::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct RewardingRatios {
    /// The percent of the epoch reward being awarded for block signing.
    pub block_signing: f64,

    /// The percent of the epoch reward being awarded for credential issuance.
    pub credential_issuance: f64,

    /// The percent of the epoch reward being awarded for credential verification.
    pub credential_verification: f64,

    /// The percent of the epoch reward given to Nym.
    pub nym: f64,
}

impl Default for RewardingRatios {
    fn default() -> Self {
        RewardingRatios {
            block_signing: 0.67,
            credential_issuance: 0.33,
            credential_verification: 0.0,
            nym: 0.0,
        }
    }
}

impl RewardingRatios {
    pub fn ensure_is_valid(&self) -> Result<(), NymRewarderError> {
        if self.block_signing + self.credential_verification + self.credential_issuance + self.nym
            != 1.0
        {
            todo!()
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NyxdScraper {
    /// Url to the websocket endpoint of a validator, for example `wss://rpc.nymtech.net/websocket`
    pub websocket_url: Url,
    // TODO: debug with everything that's currently hardcoded in the scraper
}
