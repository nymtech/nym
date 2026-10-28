// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::EpochId;

/// How many epochs before a missing one the fallback may reach for an earlier bundle.
pub const MAX_FALLBACK_LOOKBACK_EPOCHS: EpochId = 24;

pub mod storage_keys {
    pub const CONTRACT_ADMIN: &str = "contract-admin";
    pub const INITIAL_EPOCH_ID: &str = "initial-epoch-id";
    pub const LAST_SUBMISSION: &str = "last-submission";
    pub const MIXNET_CONTRACT: &str = "mixnet-contract";
    pub const AUTHORISED_COUNT: &str = "authorised-count";
    pub const AUTHORISED: &str = "authorised";
    pub const RETIRED: &str = "retired";
    /// Abbreviated: the only namespace paid once per (epoch, node).
    pub const PERFORMANCE_RESULTS: &str = "pr";
    pub const SUBMISSION_METADATA: &str = "submission-metadata";
    pub const LAST_KNOWN_EPOCH: &str = "last-known-epoch";
    pub const WEIGHTS: &str = "weights";
}
