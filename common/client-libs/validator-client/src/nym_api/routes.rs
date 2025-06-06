// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub const V1_API_VERSION: &str = "v1";
pub const V2_API_VERSION: &str = "v2";
pub const MIXNODES: &str = "mixnodes";
pub const GATEWAYS: &str = "gateways";
pub const DESCRIBED: &str = "described";
pub const BLACKLISTED: &str = "blacklisted";

pub const DETAILED: &str = "detailed";
pub const DETAILED_UNFILTERED: &str = "detailed-unfiltered";
pub const ACTIVE: &str = "active";
pub const REWARDED: &str = "rewarded";
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
    pub const ECASH_ISSUED_TICKETBOOKS_FOR: &str = "issued-ticketbooks-for";
    pub const ECASH_ISSUED_TICKETBOOKS_COUNT: &str = "issued-ticketbooks-count";
    pub const ECASH_ISSUED_TICKETBOOKS_FOR_COUNT: &str = "issued-ticketbooks-for-count";
    pub const ECASH_ISSUED_TICKETBOOKS_ON_COUNT: &str = "issued-ticketbooks-on-count";
    pub const ECASH_ISSUED_TICKETBOOKS_CHALLENGE_COMMITMENT: &str =
        "issued-ticketbooks-challenge-commitment";
    pub const ECASH_ISSUED_TICKETBOOKS_DATA: &str = "issued-ticketbooks-data";

    pub const EXPIRATION_DATE_PARAM: &str = "expiration_date";
    pub const EPOCH_ID_PARAM: &str = "epoch_id";
}

pub const NYM_NODES_ROUTES: &str = "nym-nodes";

pub use nym_nodes::*;
pub mod nym_nodes {
    pub const NYM_NODES_PERFORMANCE_HISTORY: &str = "performance-history";
    pub const NYM_NODES_PERFORMANCE: &str = "performance";
    pub const NYM_NODES_ANNOTATION: &str = "annotation";
    pub const NYM_NODES_DESCRIBED: &str = "described";
    pub const NYM_NODES_BONDED: &str = "bonded";
    pub const NYM_NODES_REWARDED_SET: &str = "rewarded-set";
    pub const NYM_NODES_REFRESH_DESCRIBED: &str = "refresh-described";
    pub const BY_ADDRESSES: &str = "by-addresses";
}

pub const STATUS_ROUTES: &str = "status";
pub const API_STATUS_ROUTES: &str = "api-status";
pub const HEALTH: &str = "health";
pub const BUILD_INFORMATION: &str = "build-information";

pub const MIXNODE: &str = "mixnode";
pub const GATEWAY: &str = "gateway";
pub const NYM_NODES: &str = "nym-nodes";

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

pub const DETAILS: &str = "details";
pub const CHAIN_STATUS: &str = "chain-status";
pub const NETWORK: &str = "network";

pub const EPOCH: &str = "epoch";

pub use epoch_routes::*;
pub mod epoch_routes {
    pub const CURRENT: &str = "current";
    pub const KEY_ROTATION_INFO: &str = "key-rotation-info";
}
