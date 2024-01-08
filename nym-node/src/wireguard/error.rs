// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WireguardError {
    #[error("the client is currently not in the process of being registered")]
    RegistrationNotInProgress,

    #[error("the client mac failed to get verified correctly")]
    MacVerificationFailure,
}
