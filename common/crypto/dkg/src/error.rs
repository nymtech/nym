// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DkgError {
    #[error("Provided set of values contained duplicate coordinate")]
    DuplicateCoordinate,

    #[error("The public key is malformed")]
    MalformedPublicKey,

    #[error("Could not solve the discrete log")]
    UnsolvableDiscreteLog,

    #[error("Received share is malformed")]
    MalformedShare,

    #[error("The share encrypted under index {0} doesn't exist")]
    UnavailableCiphertext(usize),
}
