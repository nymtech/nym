// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum UpgradeModeCheckRequest {
    /// Attempt to request upgrade mode recheck via the JWT issued as the result of
    /// global attestation.json being published
    UpgradeModeJwt { token: String },
}
