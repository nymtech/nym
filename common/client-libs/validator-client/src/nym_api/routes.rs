// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_network_defaults::NYM_API_VERSION;

pub const API_VERSION: &str = NYM_API_VERSION;
pub const MIXNODES: &str = "mixnodes";
pub const GATEWAYS: &str = "gateways";
pub const DESCRIBED: &str = "described";
pub const BLACKLISTED: &str = "blacklisted";

pub const DETAILED: &str = "detailed";
pub const DETAILED_UNFILTERED: &str = "detailed-unfiltered";
pub const ACTIVE: &str = "active";
pub const REWARDED: &str = "rewarded";
pub const DOUBLE_SPENDING_FILTER_V1: &str = "double-spending-filter-v1";

pub const ECASH_ROUTES: &str = "ecash";

pub use ecash::*;
pub mod ecash {
    pub const ECASH_BLIND_SIGN: &str = "blind-sign";
    pub const VERIFY_ECASH_TICKET: &str = "verify-ecash-ticket";
    pub const BATCH_REDEEM_ECASH_TICKETS: &str = "batch-redeem-ecash-tickets";
    pub const PARTIAL_EXPIRATION_DATE_SIGNATURES: &str = "partial-expiration-date-signatures";
    pub const GLOBAL_EXPIRATION_DATE_SIGNATURES: &str = "aggregated-expiration-date-signatures";
    pub const PARTIAL_COIN_INDICES_SIGNATURES: &str = "partial-coin-indices-signatures";
    pub const GLOBAL_COIN_INDICES_SIGNATURES: &str = "aggregated-coin-indices-signatures";
    pub const MASTER_VERIFICATION_KEY: &str = "master-verification-key";
    pub const ECASH_EPOCH_CREDENTIALS: &str = "epoch-credentials";
    pub const ECASH_ISSUED_CREDENTIAL: &str = "issued-credential";
    pub const ECASH_ISSUED_CREDENTIALS: &str = "issued-credentials";

    pub const EXPIRATION_DATE_PARAM: &str = "expiration_date";
    pub const EPOCH_ID_PARAM: &str = "epoch_id";
}

pub const STATUS_ROUTES: &str = "status";
pub const MIXNODE: &str = "mixnode";
pub const GATEWAY: &str = "gateway";

pub const CORE_STATUS_COUNT: &str = "core-status-count";
pub const SINCE_ARG: &str = "since";

pub const STATUS: &str = "status";
pub const REPORT: &str = "report";
pub const HISTORY: &str = "history";
pub const REWARD_ESTIMATION: &str = "reward-estimation";
pub const COMPUTE_REWARD_ESTIMATION: &str = "compute-reward-estimation";
pub const AVG_UPTIME: &str = "avg_uptime";
pub const STAKE_SATURATION: &str = "stake-saturation";
pub const INCLUSION_CHANCE: &str = "inclusion-probability";
pub const SUBMIT_GATEWAY: &str = "submit-gateway-monitoring-results";
pub const SUBMIT_NODE: &str = "submit-node-monitoring-results";

pub const SERVICE_PROVIDERS: &str = "services";
