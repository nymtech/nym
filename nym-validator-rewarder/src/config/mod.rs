// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::persistence::paths::ValidatorRewarderPaths;
use crate::config::r#override::ConfigOverride;
use crate::config::template::CONFIG_TEMPLATE;
use crate::error::NymRewarderError;
use crate::rewarder::ticketbook_issuance;
use cosmwasm_std::{Decimal, Uint128};
use nym_config::{
    must_get_home, read_config_from_toml_file, save_formatted_config_to_file, NymConfigTemplate,
    DEFAULT_CONFIG_DIR, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIR, NYM_DIR,
};
use nym_validator_client::nyxd::{AccountId, Coin};
use nyxd_scraper::PruningOptions;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info};
use url::Url;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub mod r#override;
pub mod persistence;
mod template;

const DEFAULT_REWARDER_DIR: &str = "validators-rewarder";

#[allow(clippy::inconsistent_digit_grouping)]
const DEFAULT_DAILY_REWARDING_BUDGET: u128 = 24000_000000;

// #[allow(clippy::inconsistent_digit_grouping)]
// const DEFAULT_MIX_REWARDING_BUDGET: u128 = 1000_000000;
const DEFAULT_REWARDING_DENOM: &str = "unym";

const DEFAULT_BLOCK_SIGNING_EPOCH_DURATION: Duration = Duration::from_secs(60 * 60);
const DEFAULT_TICKETBOOK_ISSUANCE_MIN_VALIDATE: usize = 10;
const DEFAULT_TICKETBOOK_ISSUANCE_SAMPLING_RATE: f64 = 0.10;
const DEFAULT_TICKETBOOK_ISSUANCE_FULL_VERIFICATION_RATIO: f64 = 0.60;

// 'worst' case scenario
pub const TYPICAL_BLOCK_TIME: f32 = 5.;

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
    #[serde(default)]
    pub rewarding: Rewarding,

    #[zeroize(skip)]
    #[serde(default)]
    pub block_signing: BlockSigning,

    #[zeroize(skip)]
    #[serde(default)]
    pub ticketbook_issuance: TicketbookIssuance,

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
    pub fn new(mnemonic: bip39::Mnemonic, websocket_url: Url, nyxd_url: Url) -> Self {
        Config {
            save_path: None,
            rewarding: Rewarding::default(),
            block_signing: Default::default(),
            ticketbook_issuance: TicketbookIssuance::default(),
            nyxd_scraper: NyxdScraper {
                websocket_url,
                pruning: Default::default(),
            },
            base: Base {
                upstream_nyxd: nyxd_url,
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
            pruning_options: self.nyxd_scraper.pruning,
        }
    }

    pub fn verification_config(&self) -> ticketbook_issuance::VerificationConfig {
        ticketbook_issuance::VerificationConfig {
            min_validate_per_issuer: self.ticketbook_issuance.min_validate_per_issuer,
            sampling_rate: self.ticketbook_issuance.sampling_rate,
            full_verification_ratio: self.ticketbook_issuance.full_verification_ratio,
        }
    }

    pub fn validate(&self) -> Result<(), NymRewarderError> {
        self.rewarding.ratios.validate()?;
        self.nyxd_scraper
            .validate(self.block_signing.epoch_duration)?;
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
        loaded.validate()?;
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

    pub fn will_attempt_to_send_rewards(&self) -> bool {
        (self.block_signing.enabled && !self.block_signing.monitor_only)
            || (self.ticketbook_issuance.enabled && !self.ticketbook_issuance.monitor_only)
    }

    /// Returns the total rewarding budget for block signing for given epoch
    pub fn block_signing_epoch_budget(&self) -> Coin {
        // it doesn't have to be exact to sub micronym precision
        let daily_block_signing_budget =
            self.rewarding.daily_budget.amount as f64 * self.rewarding.ratios.block_signing;

        // how many epochs per day are there?
        let epoch_ratio = self.block_signing.epoch_duration.as_secs_f64() / (24. * 60. * 60.);

        let epoch_budget = (daily_block_signing_budget * epoch_ratio) as u128;
        Coin::new(epoch_budget, &self.rewarding.daily_budget.denom)
    }

    /// Returns the total rewarding budget for ticketbook issuance for given day
    pub fn ticketbook_issuance_daily_budget(&self) -> Coin {
        // it doesn't have to be exact to sub micronym precision
        let daily_ticketbook_issuance_budget =
            self.rewarding.daily_budget.amount as f64 * self.rewarding.ratios.ticketbook_issuance;

        let ticketbook_issuance_budget = daily_ticketbook_issuance_budget as u128;
        Coin::new(
            ticketbook_issuance_budget,
            &self.rewarding.daily_budget.denom,
        )
    }

    /// Returns the total rewarding budget for ticketbook issuance for an individual operator for given day
    pub fn ticketbook_per_operator_daily_budget(&self) -> Coin {
        let ticketbook_total_budget = self.ticketbook_issuance_daily_budget();

        let whitelist_size = self.ticketbook_issuance.whitelist.len();

        let amount = if self.ticketbook_issuance.whitelist.is_empty() {
            Uint128::zero()
        } else {
            Uint128::new(ticketbook_total_budget.amount)
                * Decimal::from_ratio(1u32, whitelist_size as u64)
        };

        let per_operator = Coin::new(amount.u128(), &ticketbook_total_budget.denom);

        let total_budget = &self.rewarding.daily_budget;
        info!("ISSUANCE BUDGET: with the total daily budget of {total_budget} ({ticketbook_total_budget} for ticketbook issuance) and with whitelist size of {whitelist_size}, the per operator budget is set to {per_operator}");

        per_operator
    }
}

#[derive(Debug, Deserialize, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Base {
    /// Url to the upstream instance of nyxd to use for any queries and rewarding.
    #[zeroize(skip)]
    pub upstream_nyxd: Url,

    /// Mnemonic to the nyx account distributing the rewards
    pub(crate) mnemonic: bip39::Mnemonic,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Rewarding {
    /// Specifies total budget for a 24h period
    #[serde_as(as = "DisplayFromStr")]
    pub daily_budget: Coin,

    pub ratios: RewardingRatios,
}

impl Default for Rewarding {
    fn default() -> Self {
        Rewarding {
            daily_budget: Coin::new(DEFAULT_DAILY_REWARDING_BUDGET, DEFAULT_REWARDING_DENOM),
            ratios: RewardingRatios::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct RewardingRatios {
    /// The percent of the epoch reward being awarded for block signing.
    pub block_signing: f64,

    /// The percent of the epoch reward being awarded for ticketbook issuance.
    #[serde(alias = "credential_issuance")]
    pub ticketbook_issuance: f64,

    /// The percent of the epoch reward being awarded for ticketbook verification.
    #[serde(alias = "credential_verification")]
    pub ticketbook_verification: f64,
    // /// The percent of the epoch reward given to Nym.
    // pub nym: f64,
}

impl Default for RewardingRatios {
    fn default() -> Self {
        RewardingRatios {
            block_signing: 0.67,
            ticketbook_issuance: 0.33,
            ticketbook_verification: 0.0,
            // nym: 0.0,
        }
    }
}

impl RewardingRatios {
    pub fn validate(&self) -> Result<(), NymRewarderError> {
        if self.block_signing + self.ticketbook_verification + self.ticketbook_issuance != 1.0 {
            return Err(NymRewarderError::InvalidRewardingRatios { ratios: *self });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NyxdScraper {
    /// Url to the websocket endpoint of a validator, for example `wss://rpc.nymtech.net/websocket`
    pub websocket_url: Url,

    /// Defines the pruning options, if applicable, to be used by the underlying scraper.
    // if the value is missing, use `nothing` pruning as this was the past behaviour
    #[serde(default = "PruningOptions::nothing")]
    pub pruning: PruningOptions,
    // TODO: debug with everything that's currently hardcoded in the scraper
}

impl NyxdScraper {
    pub fn validate(&self, epoch_duration: Duration) -> Result<(), NymRewarderError> {
        // basic, sanity check, of pruning
        self.pruning.validate()?;

        if self.pruning.strategy.is_nothing() {
            return Ok(());
        }

        // rewarder-specific validation:
        if self.pruning.strategy.is_everything() {
            return Err(NymRewarderError::EverythingPruningStrategy);
        }

        if self.pruning.strategy.is_custom() {
            let min_to_keep =
                (epoch_duration.as_secs_f32() / TYPICAL_BLOCK_TIME * 1.5).ceil() as u32;

            if self.pruning.strategy_keep_recent() < min_to_keep {
                return Err(NymRewarderError::TooSmallKeepRecent {
                    min_to_keep,
                    keep_recent: self.pruning.strategy_keep_recent(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlockSigning {
    /// Specifies whether rewards for block signing is enabled.
    pub enabled: bool,

    /// Duration of block signing epoch.
    #[serde(with = "humantime_serde")]
    pub epoch_duration: Duration,

    /// Specifies whether to only monitor and not send rewards.
    pub monitor_only: bool,

    /// List of validators that will receive rewards for block signing.
    /// If not on the list, the validator will be treated as if it had 0 voting power.
    pub whitelist: Vec<AccountId>,
}

impl Default for BlockSigning {
    fn default() -> Self {
        BlockSigning {
            enabled: true,
            epoch_duration: DEFAULT_BLOCK_SIGNING_EPOCH_DURATION,
            monitor_only: false,
            whitelist: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TicketbookIssuance {
    /// Specifies whether rewarding for ticketbook issuance is enabled.
    pub enabled: bool,

    /// Specifies whether to only monitor and not send rewards.
    pub monitor_only: bool,

    /// Defines the minimum number of ticketbooks the rewarder will validate
    /// regardless of the sampling rate
    #[serde(default = "default_ticketbook_issuance_min_validate")]
    pub min_validate_per_issuer: usize,

    /// The sampling rate of the issued ticketbooks
    #[serde(default = "default_ticketbook_issuance_sampling_rate")]
    pub sampling_rate: f64,

    /// Ratio of issuers that will undergo full verification as opposed to being let through.
    #[serde(default = "default_ticketbook_issuance_full_verification_ratio")]
    pub full_verification_ratio: f64,

    /// List of validators that will receive rewards for ticketbook issuance.
    /// If not on the list, the validator will be treated as if it hadn't issued a single ticketbook.
    pub whitelist: Vec<AccountId>,
}

fn default_ticketbook_issuance_min_validate() -> usize {
    TicketbookIssuance::default().min_validate_per_issuer
}

fn default_ticketbook_issuance_sampling_rate() -> f64 {
    TicketbookIssuance::default().sampling_rate
}

fn default_ticketbook_issuance_full_verification_ratio() -> f64 {
    TicketbookIssuance::default().full_verification_ratio
}

impl Default for TicketbookIssuance {
    fn default() -> Self {
        TicketbookIssuance {
            enabled: false,
            monitor_only: false,
            min_validate_per_issuer: DEFAULT_TICKETBOOK_ISSUANCE_MIN_VALIDATE,
            sampling_rate: DEFAULT_TICKETBOOK_ISSUANCE_SAMPLING_RATE,
            full_verification_ratio: DEFAULT_TICKETBOOK_ISSUANCE_FULL_VERIFICATION_RATIO,
            whitelist: vec![],
        }
    }
}
