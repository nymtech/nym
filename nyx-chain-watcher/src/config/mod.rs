// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::template::CONFIG_TEMPLATE;
use nym_bin_common::logging::LoggingSettings;
use nym_config::{
    must_get_home, read_config_from_toml_file, save_unformatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, error};

pub(crate) mod payments_watcher;
mod template;

pub use crate::config::payments_watcher::PaymentWatcherConfig;
use crate::error::NyxChainWatcherError;

const DEFAULT_NYM_CHAIN_WATCHER_DIR: &str = "nym-chain-watcher";

pub(crate) const DEFAULT_NYM_CHAIN_WATCHER_DB_FILENAME: &str = "nyx_chain_watcher.sqlite";
pub(crate) const DEFAULT_NYM_CHAIN_SCRAPER_HISTORY_DB_FILENAME: &str = "chain_history.sqlite";

/// Derive default path to nym-chain-watcher's config directory.
/// It should get resolved to `$HOME/.nym/nym-chain-watcher/config`
pub fn default_config_directory() -> PathBuf {
    must_get_home()
        .join(NYM_DIR)
        .join(DEFAULT_NYM_CHAIN_WATCHER_DIR)
        .join(DEFAULT_CONFIG_DIR)
}

/// Derive default path to nym-chain-watcher's config file.
/// It should get resolved to `$HOME/.nym/nym-chain-watcher/config/config.toml`
pub fn default_config_filepath() -> PathBuf {
    default_config_directory().join(DEFAULT_CONFIG_FILENAME)
}

pub struct ConfigBuilder {
    pub config_path: PathBuf,

    pub data_dir: PathBuf,

    pub db_path: Option<String>,

    pub chain_scraper_db_path: Option<String>,

    pub payment_watcher_config: Option<PaymentWatcherConfig>,

    pub logging: Option<LoggingSettings>,

    pub bearer_token: Option<String>,
}

impl ConfigBuilder {
    pub fn new(config_path: PathBuf, data_dir: PathBuf) -> Self {
        ConfigBuilder {
            config_path,
            data_dir,
            payment_watcher_config: None,
            logging: None,
            db_path: None,
            chain_scraper_db_path: None,
            bearer_token: None,
        }
    }

    pub fn with_record_bearer_token(mut self, token: String) -> Self {
        self.bearer_token = Some(token);
        self
    }

    pub fn with_db_path(mut self, db_path: String) -> Self {
        self.db_path = Some(db_path);
        self
    }

    pub fn with_chain_scraper_db_path(mut self, chain_scraper_db_path: String) -> Self {
        self.chain_scraper_db_path = Some(chain_scraper_db_path);
        self
    }

    #[allow(dead_code)]
    pub fn with_payment_watcher_config(
        mut self,
        payment_watcher_config: impl Into<PaymentWatcherConfig>,
    ) -> Self {
        self.payment_watcher_config = Some(payment_watcher_config.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_logging(mut self, section: impl Into<Option<LoggingSettings>>) -> Self {
        self.logging = section.into();
        self
    }

    pub fn build(self) -> Config {
        Config {
            logging: self.logging.unwrap_or_default(),
            save_path: Some(self.config_path),
            payment_watcher_config: self.payment_watcher_config,
            data_dir: self.data_dir,
            db_path: self.db_path,
            chain_scraper_db_path: self.chain_scraper_db_path,
            bearer_token: self.bearer_token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // additional metadata holding on-disk location of this config file
    #[serde(skip)]
    pub(crate) save_path: Option<PathBuf>,

    #[serde(skip)]
    pub(crate) data_dir: PathBuf,

    #[serde(skip)]
    db_path: Option<String>,

    #[serde(skip)]
    chain_scraper_db_path: Option<String>,

    pub payment_watcher_config: Option<PaymentWatcherConfig>,

    pub bearer_token: Option<String>,

    #[serde(default)]
    pub logging: LoggingSettings,

}

impl NymConfigTemplate for Config {
    fn template(&self) -> &'static str {
        CONFIG_TEMPLATE
    }
}

impl Config {
    #[allow(unused)]
    pub fn save(&self) -> Result<(), NyxChainWatcherError> {
        let save_location = self.save_location();
        debug!(
            "attempting to save config file to '{}'",
            save_location.display()
        );
        save_unformatted_config_to_file(self, &save_location).map_err(|source| {
            NyxChainWatcherError::UnformattedConfigSaveFailure {
                path: save_location,
                source,
            }
        })
    }

    #[allow(unused)]
    pub fn save_location(&self) -> PathBuf {
        self.save_path
            .clone()
            .unwrap_or(self.default_save_location())
    }

    #[allow(unused)]
    pub fn default_save_location(&self) -> PathBuf {
        default_config_filepath()
    }

    pub fn default_data_directory<P: AsRef<Path>>(
        config_path: P,
    ) -> Result<PathBuf, NyxChainWatcherError> {
        let config_path = config_path.as_ref();

        // we got a proper path to the .toml file
        let Some(config_dir) = config_path.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_path.display()
            );
            return Err(NyxChainWatcherError::DataDirDerivationFailure);
        };

        let Some(config_dir_name) = config_dir.file_name() else {
            error!(
                "could not obtain parent directory name of '{}'. Have you used relative paths?",
                config_path.display()
            );
            return Err(NyxChainWatcherError::DataDirDerivationFailure);
        };

        if config_dir_name != DEFAULT_CONFIG_DIR {
            error!(
                "the parent directory of '{}' ({}) is not {DEFAULT_CONFIG_DIR}. currently this is not supported",
                config_path.display(), config_dir_name.to_str().unwrap_or("UNKNOWN")
            );
            return Err(NyxChainWatcherError::DataDirDerivationFailure);
        }

        let Some(node_dir) = config_dir.parent() else {
            error!(
                "'{}' does not have a parent directory. Have you pointed to the fs root?",
                config_dir.display()
            );
            return Err(NyxChainWatcherError::DataDirDerivationFailure);
        };

        Ok(node_dir.join(DEFAULT_DATA_DIR))
    }

    pub fn database_path(&self) -> String {
        self.db_path.clone().unwrap_or_else(|| {
            let mut path = self.data_dir.clone().to_path_buf();
            path.push(DEFAULT_NYM_CHAIN_WATCHER_DB_FILENAME);
            path.to_str()
                .unwrap_or(DEFAULT_NYM_CHAIN_WATCHER_DB_FILENAME)
                .to_string()
        })
    }

    pub fn chain_scraper_database_path(&self) -> String {
        self.chain_scraper_db_path.clone().unwrap_or_else(|| {
            let mut path = self.data_dir.clone().to_path_buf();
            path.push(DEFAULT_NYM_CHAIN_SCRAPER_HISTORY_DB_FILENAME);
            path.to_str()
                .unwrap_or(DEFAULT_NYM_CHAIN_SCRAPER_HISTORY_DB_FILENAME)
                .to_string()
        })
    }

    // simple wrapper that reads config file and assigns path location
    fn read_from_path<P: AsRef<Path>>(path: P, data_dir: P) -> Result<Self, NyxChainWatcherError> {
        let path = path.as_ref();
        let data_dir = data_dir.as_ref();
        let mut loaded: Config = read_config_from_toml_file(path).map_err(|source| {
            NyxChainWatcherError::ConfigLoadFailure {
                path: path.to_path_buf(),
                source,
            }
        })?;
        loaded.data_dir = data_dir.to_path_buf();
        loaded.save_path = Some(path.to_path_buf());
        debug!("loaded config file from {}", path.display());
        Ok(loaded)
    }

    #[allow(unused)]
    pub fn read_from_toml_file<P: AsRef<Path>>(
        path: P,
        data_dir: P,
    ) -> Result<Self, NyxChainWatcherError> {
        Self::read_from_path(path, data_dir)
    }

    pub fn read_from_toml_file_in_default_location() -> Result<Self, NyxChainWatcherError> {
        let config_path = default_config_filepath();
        let data_dir = Config::default_data_directory(&config_path)?;
        Self::read_from_path(config_path, data_dir)
    }
}
