// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

// approximately 1 week (assuming 5s per block)
// i.e. approximately quarter of the interval (there are 3600 * 60 * 7 = 604800 seconds in a week, i.e. ~604800 / 5 = 120960 blocks)
pub const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 120960;

pub const INTERVAL_REWARD_PERCENT: u8 = 2; // Used to calculate interval reward pool
pub const SYBIL_RESISTANCE_PERCENT: u8 = 30;
pub const ACTIVE_SET_WORK_FACTOR: u8 = 10;

// TODO: this, in theory, represents "epoch" length.
// However, since the blocktime is not EXACTLY 5s, we can't really guarantee 720 epochs in interval
// and we can't change this easily to `Duration`, because then the entire rewarded set storage
// would be messed up... (as we look up stuff "by blocks")
pub const REWARDED_SET_REFRESH_BLOCKS: u64 = 720; // with blocktime being approximately 5s, it should be roughly 1h
pub const EPOCHS_IN_INTERVAL: u64 = 720; // Hours in a month

pub const REWARDING_INTERVAL_LENGTH: Duration = Duration::from_secs(60 * 60); // 3600s, i.e. 1h
