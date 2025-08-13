// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod attestation;
pub(crate) mod error;
pub(crate) mod jwt;

pub use attestation::{
    attempt_retrieve_attestation, generate_new_attestation,
    generate_new_attestation_with_starting_time, UpgradeModeAttestation,
};
pub use error::UpgradeModeCheckError;
pub use jwt::{generate_jwt_for_upgrade_mode_attestation, validate_upgrade_mode_jwt};
