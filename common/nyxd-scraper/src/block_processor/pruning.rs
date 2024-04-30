// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ScraperError;
use serde::{Deserialize, Serialize};

pub const DEFAULT_PRUNING_KEEP_RECENT: usize = 362880;
pub const DEFAULT_PRUNING_INTERVAL: u32 = 10;
pub const EVERYTHING_PRUNING_KEEP_RECENT: usize = 2;
pub const EVERYTHING_PRUNING_INTERVAL: u32 = 10;

/// We follow cosmos-sdk pruning strategies for convenience’s sake.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PruningOptions {
    /// keep_recent defines how many recent heights to keep on disk.
    pub keep_recent: usize,

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
}

/*
func (po PruningOptions) Validate() error {
    if po.Strategy == PruningNothing {
        return nil
    }
    if po.Interval == 0 {
        return ErrPruningIntervalZero
    }
    if po.Interval < pruneEverythingInterval {
        return ErrPruningIntervalTooSmall
    }
    if po.KeepRecent < pruneEverythingKeepRecent {
        return ErrPruningKeepRecentTooSmall
    }
    return nil
}
 */

impl Default for PruningOptions {
    fn default() -> Self {
        PruningOptions {
            keep_recent: DEFAULT_PRUNING_KEEP_RECENT,
            interval: DEFAULT_PRUNING_INTERVAL,
            strategy: Default::default(),
        }
    }
}

/*


## Strategies

The strategies are configured in `app.toml`, with the format `pruning = "<strategy>"` where the options are:

* `default`: only the last 362,880 states(approximately 3.5 weeks worth of state) are kept; pruning at 10 block intervals
* `nothing`: all historic states will be saved, nothing will be deleted (i.e. archiving node)
* `everything`: 2 latest states will be kept; pruning at 10 block intervals.
* `custom`: allow pruning options to be manually specified through 'pruning-keep-recent', and 'pruning-interval'

If no strategy is given to the BaseApp, `nothing` is selected. However, we perform validation on the CLI layer to require these to be always set in the config file.

## Custom Pruning

These are applied if and only if the pruning strategy is custom:

* `pruning-keep-recent`: N means to keep all of the last N states
* `pruning-interval`: N means to delete old states from disk every Nth block.

 */

/*

const (
    pruneEverythingKeepRecent = 2
    pruneEverythingInterval   = 10
)

var (
    ErrPruningIntervalZero       = errors.New("'pruning-interval' must not be 0. If you want to disable pruning, select pruning = \"nothing\"")
    ErrPruningIntervalTooSmall   = fmt.Errorf("'pruning-interval' must not be less than %d. For the most aggressive pruning, select pruning = \"everything\"", pruneEverythingInterval)
    ErrPruningKeepRecentTooSmall = fmt.Errorf("'pruning-keep-recent' must not be less than %d. For the most aggressive pruning, select pruning = \"everything\"", pruneEverythingKeepRecent)
)

func NewPruningOptions(pruningStrategy PruningStrategy) PruningOptions {
    switch pruningStrategy {
    case PruningDefault:
        return PruningOptions{
            KeepRecent: 362880,
            Interval:   10,
            Strategy:   PruningDefault,
        }
    case PruningEverything:
        return PruningOptions{
            KeepRecent: pruneEverythingKeepRecent,
            Interval:   pruneEverythingInterval,
            Strategy:   PruningEverything,
        }
    case PruningNothing:
        return PruningOptions{
            KeepRecent: 0,
            Interval:   0,
            Strategy:   PruningNothing,
        }
    default:
        return PruningOptions{
            Strategy: PruningCustom,
        }
    }
}

*/
