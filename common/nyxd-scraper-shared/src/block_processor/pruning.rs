// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ScraperError;
use serde::{Deserialize, Serialize};

pub const DEFAULT_PRUNING_KEEP_RECENT: u32 = 362880;
pub const DEFAULT_PRUNING_INTERVAL: u32 = 10;
pub const EVERYTHING_PRUNING_KEEP_RECENT: u32 = 2;
pub const EVERYTHING_PRUNING_INTERVAL: u32 = 10;

/// We follow cosmos-sdk pruning strategies for convenience’s sake.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PruningStrategy {
    /// 'Default' strategy defines a pruning strategy where the last 362880 heights are
    /// kept where to-be pruned heights are pruned at every 10th height.
    /// The last 362880 heights are kept(approximately 3.5 weeks worth of state) assuming the typical
    /// block time is 6s. If these values do not match the applications' requirements, use the "custom" option.
    #[default]
    Default,

    /// 'Everything' strategy defines a pruning strategy where all committed heights are
    /// deleted, storing only the current height and last 2 states. To-be pruned heights are
    /// pruned at every 10th height.
    Everything,

    /// 'Nothing' strategy defines a pruning strategy where all heights are kept on disk.
    Nothing,

    /// 'Custom' strategy defines a pruning strategy where the user specifies the pruning.
    Custom,
}

impl PruningStrategy {
    pub fn is_custom(&self) -> bool {
        matches!(self, PruningStrategy::Custom)
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, PruningStrategy::Nothing)
    }

    pub fn is_everything(&self) -> bool {
        matches!(self, PruningStrategy::Everything)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PruningOptions {
    /// keep_recent defines how many recent heights to keep on disk.
    pub keep_recent: u32,

    /// interval defines the frequency of removing the pruned heights from the disk.
    pub interval: u32,

    /// strategy defines the currently used kind of [PruningStrategy].
    pub strategy: PruningStrategy,
}

impl PruningOptions {
    pub fn validate(&self) -> Result<(), ScraperError> {
        // if strategy is not set to custom, other options are meaningless since they won't be applied
        if !self.strategy.is_custom() {
            return Ok(());
        }

        if self.interval == 0 {
            return Err(ScraperError::ZeroPruningInterval);
        }

        if self.interval < EVERYTHING_PRUNING_INTERVAL {
            return Err(ScraperError::TooSmallPruningInterval {
                interval: self.interval,
            });
        }

        if self.keep_recent < EVERYTHING_PRUNING_KEEP_RECENT {
            return Err(ScraperError::TooSmallKeepRecent {
                keep_recent: self.keep_recent,
            });
        }

        Ok(())
    }

    pub fn nothing() -> Self {
        PruningOptions {
            keep_recent: 0,
            interval: 0,
            strategy: PruningStrategy::Nothing,
        }
    }

    pub fn strategy_interval(&self) -> u32 {
        match self.strategy {
            PruningStrategy::Default => DEFAULT_PRUNING_INTERVAL,
            PruningStrategy::Everything => EVERYTHING_PRUNING_INTERVAL,
            PruningStrategy::Nothing => 0,
            PruningStrategy::Custom => self.interval,
        }
    }

    pub fn strategy_keep_recent(&self) -> u32 {
        match self.strategy {
            PruningStrategy::Default => DEFAULT_PRUNING_KEEP_RECENT,
            PruningStrategy::Everything => EVERYTHING_PRUNING_KEEP_RECENT,
            PruningStrategy::Nothing => 0,
            PruningStrategy::Custom => self.keep_recent,
        }
    }
}

impl Default for PruningOptions {
    fn default() -> Self {
        PruningOptions {
            keep_recent: DEFAULT_PRUNING_KEEP_RECENT,
            interval: DEFAULT_PRUNING_INTERVAL,
            strategy: Default::default(),
        }
    }
}
