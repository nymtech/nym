// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod attestation;
pub(crate) mod error;
pub(crate) mod jwt;

pub use attestation::{
    UpgradeModeAttestation, generate_new_attestation, generate_new_attestation_with_starting_time,
};
pub use error::UpgradeModeCheckError;
pub use jwt::{
    CREDENTIAL_PROXY_JWT_ISSUER, generate_jwt_for_upgrade_mode_attestation,
    try_decode_upgrade_mode_jwt_claims, validate_upgrade_mode_jwt,
};

#[cfg(not(target_arch = "wasm32"))]
pub use attestation::attempt_retrieve_attestation;

pub const UPGRADE_MODE_CREDENTIAL_TYPE: &str = "upgrade_mode_jwt";
