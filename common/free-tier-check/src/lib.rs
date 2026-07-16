// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod error;
pub(crate) mod jwt;

pub use error::FreeTierCheckError;
pub use jwt::{
    CREDENTIAL_PROXY_JWT_ISSUER, FreeTierClaims, FreeTierPurpose, generate_free_tier_jwt,
    validate_free_tier_jwt,
};

/// Credential kind string for free-tier capability tokens.
pub const FREE_TIER_CREDENTIAL_TYPE: &str = "free_tier_jwt";
