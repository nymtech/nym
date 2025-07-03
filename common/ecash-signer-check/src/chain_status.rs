// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::models::ChainStatusResponse;

// Dorina
pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 51);

#[derive(Debug)]
pub enum LocalChainStatus {
    /// The API, even though it reports correct version, did not response to the status query
    Unreachable,

    /// The API is running an outdated version that does not expose the required endpoint
    Outdated,

    /// Response to the status query
    // unfortunately this response is not signed, but it's not the end of the world
    Reachable { response: Box<ChainStatusResponse> },
}
