// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// approximately 1 epoch (assuming 5s per block)
// pub const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 720;
//
// pub const INTERVAL_REWARD_PERCENT: u8 = 2; // Used to calculate interval reward pool
// pub const SYBIL_RESISTANCE_PERCENT: u8 = 30;
// pub const ACTIVE_SET_WORK_FACTOR: u8 = 10;
//
// // TODO: this, in theory, represents "epoch" length.
// // However, since the blocktime is not EXACTLY 5s, we can't really guarantee 720 epochs in interval
// // and we can't change this easily to `Duration`, because then the entire rewarded set storage
// // would be messed up... (as we look up stuff "by blocks")
// pub const REWARDED_SET_REFRESH_BLOCKS: u64 = 720; // with blocktime being approximately 5s, it should be roughly 1h
// pub const INTERVAL_SECONDS: u64 = 86400 * 30; // 30 days
// pub const DEFAULT_OPERATOR_INTERVAL_COST: u64 = 40_000_000;

// retrieval limits
pub const GATEWAY_BOND_DEFAULT_RETRIEVAL_LIMIT: u32 = 50;
pub const GATEWAY_BOND_MAX_RETRIEVAL_LIMIT: u32 = 75;

// storage keys
