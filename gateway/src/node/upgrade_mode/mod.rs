// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

pub(crate) mod common_state;
pub(crate) mod watcher;

#[derive(Debug, Error)]
pub enum UpgradeModeEnableError {
    #[error("too soon to perform another upgrade mode attestation check")]
    TooManyRecheckRequests,

    #[error("provided upgrade mode JWT is invalid: {0}")]
    InvalidUpgradeModeJWT(#[from] nym_upgrade_mode_check::UpgradeModeCheckError),

    #[error("the upgrade mode attestation does not appear to have been published")]
    AttestationNotPublished,

    #[error("the provided upgrade mode attestation is different from the published one")]
    MismatchedUpgradeModeAttestation,
}
