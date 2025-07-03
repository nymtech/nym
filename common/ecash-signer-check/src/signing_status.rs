// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::ecash::models::EcashSignerStatusResponse;

// Emmental
pub(crate) const MINIMUM_VERSION: semver::Version = semver::Version::new(1, 1, 62);

#[derive(Debug)]
pub enum SigningStatus {
    /// The API, even though it reports correct version, did not response to the status query
    Unreachable,

    /// The API is running an outdated version that does not expose the required endpoint
    Outdated,

    /// Response to the status query
    Reachable { response: EcashSignerStatusResponse },
}
