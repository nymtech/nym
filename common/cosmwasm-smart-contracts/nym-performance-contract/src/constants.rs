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

    /// The full LtHash accumulator state, `nym_lthash::DIGEST_LEN` raw bytes.
    ///
    /// These bytes are what a client obtains an ICS23 proof for, since CosmWasm smart queries
    /// carry none, so three things about this key are load-bearing and must never change:
    ///
    ///   - it is used **verbatim**, as `store.set(DIGEST_STATE.as_bytes(), ..)`, not through a
    ///     `cw-storage-plus` `Item`. There is no length prefix and no namespacing, so the proven
    ///     key is exactly these bytes appended to the contract's storage prefix (see
    ///     `contract_storage_key` in `nym-validator-client`);
    ///   - the value is the accumulator itself, never its 32-byte collapse. A verifier
    ///     reconstructs an `LtHash16` from `DIGEST_LEN` bytes here and would reject a 32-byte
    ///     value on length;
    ///   - the key never changes, across migrations included. A client pins it in order to prove
    ///     against it, so renaming it silently breaks every verifier rather than failing loudly.
    ///
    /// [`crate::QueryMsg::Digest`] serves the collapse of this same value as an unproven
    /// convenience for consumers that only need to compare digests.
    pub const DIGEST_STATE: &str = "digest_state";
}
