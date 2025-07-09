// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::models::ChainStatusResponse;
use std::time::Duration;
use time::OffsetDateTime;

// Dorina
pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 51);
const CHAIN_STALL_THRESHOLD: Duration = Duration::from_secs(5 * 60);

#[derive(Debug)]
pub enum LocalChainStatus {
    /// The API, even though it reports correct version, did not response to the status query
    Unreachable,

    /// The API is running an outdated version that does not expose the required endpoint
    Outdated,

    /// Response to the [legacy] status query
    ReachableLegacy { response: Box<ChainStatusResponse> },
    // Reachable {
    //     response: (),
    // },
}

impl LocalChainStatus {
    pub fn available(&self) -> bool {
        let LocalChainStatus::ReachableLegacy { response } = self else {
            return false;
        };

        let now = OffsetDateTime::now_utc();
        let block_time: OffsetDateTime = response.status.latest_block.block.header.time.into();
        let diff = now - block_time;
        diff <= CHAIN_STALL_THRESHOLD
    }
}
