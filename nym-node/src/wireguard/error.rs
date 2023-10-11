// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::AddrParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WireguardError {
    #[error("the provided client socket address ('{raw}') was malformed: {source}")]
    MalformedClientSocketAddress {
        raw: String,
        #[source]
        source: AddrParseError,
    },

    #[error("the client is currently not in the process of being registered")]
    RegistrationNotInProgress,

    #[error("the client mac failed to get verified correctly")]
    MacVerificationFailure,
}
