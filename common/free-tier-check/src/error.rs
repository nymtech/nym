// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FreeTierCheckError {
    #[error("failed to verify the free-tier jwt: {source}")]
    JwtVerificationFailure { source: jwt_simple::Error },
}
