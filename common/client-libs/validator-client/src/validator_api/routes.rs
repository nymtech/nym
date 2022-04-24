// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use network_defaults::VALIDATOR_API_VERSION;

pub const API_VERSION: &str = VALIDATOR_API_VERSION;
pub const MIXNODES: &str = "mixnodes";
pub const GATEWAYS: &str = "gateways";

pub const ACTIVE: &str = "active";
pub const REWARDED: &str = "rewarded";

pub const COCONUT_ROUTES: &str = "coconut";
pub const BANDWIDTH: &str = "bandwidth";

pub const COCONUT_BLIND_SIGN: &str = "blind-sign";
pub const COCONUT_PARTIAL_BANDWIDTH_CREDENTIAL: &str = "partial-bandwidth-credential";
pub const COCONUT_VERIFICATION_KEY: &str = "verification-key";

pub const STATUS_ROUTES: &str = "status";
pub const MIXNODE: &str = "mixnode";
pub const GATEWAY: &str = "gateway";

pub const CORE_STATUS_COUNT: &str = "core-status-count";
pub const SINCE_ARG: &str = "since";

pub const STATUS: &str = "status";
pub const REWARD_ESTIMATION: &str = "reward-estimation";
pub const AVG_UPTIME: &str = "avg_uptime";
pub const STAKE_SATURATION: &str = "stake-saturation";
pub const INCLUSION_CHANCE: &str = "inclusion-probability";
