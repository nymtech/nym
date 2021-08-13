// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoconutInterfaceError {
    #[error("could not parse validator URL: {source}")]
    UrlParsingError {
        #[from]
        source: url::ParseError,
    },

    #[error("could not aggregate verification key: {0}")]
    AggregateVerificationKeyError(coconut_rs::CoconutError),

    #[error("could not prove credential: {0}")]
    ProveCredentialError(coconut_rs::CoconutError),

    #[error("got invalid signature index: {0}")]
    InvalidSignatureIdx(usize),

    #[error("got too many total attributes(public + private): {0} received, {1} is the maximum")]
    TooManyTotalAttributes(usize, u32),
}
