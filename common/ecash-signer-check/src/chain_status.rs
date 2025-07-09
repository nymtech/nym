// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::status::STALE_RESPONSE_THRESHOLD;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::models::{ChainBlocksStatusResponse, ChainStatusResponse};
use std::time::Duration;
use time::OffsetDateTime;
use tracing::warn;

// Dorina
pub(crate) const MINIMUM_VERSION_LEGACY: semver::Version = semver::Version::new(1, 1, 51);

// Emmental
pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 62);

const CHAIN_STALL_THRESHOLD: Duration = Duration::from_secs(5 * 60);

#[derive(Debug)]
pub enum LocalChainStatus {
    /// The API, even though it reports correct version, did not response to the status query
    Unreachable,

    /// The API is running an outdated version that does not expose the required endpoint
    Outdated,

    /// Response to the legacy (unsigned) status query
    ReachableLegacy { response: Box<ChainStatusResponse> },

    /// Response to the current (signed) status query
    Reachable {
        response: Box<ChainBlocksStatusResponse>,
    },
}

impl LocalChainStatus {
    pub fn available(&self, pub_key: ed25519::PublicKey) -> bool {
        let now = OffsetDateTime::now_utc();
        match self {
            LocalChainStatus::Unreachable | LocalChainStatus::Outdated => false,
            LocalChainStatus::ReachableLegacy { response } => response
                .status
                .stall_status(now, CHAIN_STALL_THRESHOLD)
                .is_synced(),
            LocalChainStatus::Reachable { response } => {
                if !response.verify_signature(&pub_key) {
                    warn!("failed signature verification on chain status response");
                    return false;
                }

                // we rely on information provided from the api itself AS LONG AS it's not too outdated
                if response.body.current_time + STALE_RESPONSE_THRESHOLD < now {
                    return false;
                }
                response.body.chain_status.is_synced()
            }
        }
    }
}
