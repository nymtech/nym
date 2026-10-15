// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::EpochId;

/// How many epochs the fallback may reach below the newest epoch with any data.
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

    /// The raw key of the full LtHash accumulator, written through
    /// `nym_contracts_common::digest::DigestItem`, whose doc says why the key and its raw format
    /// must never change. [`crate::QueryMsg::Digest`] serves the collapse of the same value.
    pub const DIGEST_STATE: &str = "digest_state";
}
