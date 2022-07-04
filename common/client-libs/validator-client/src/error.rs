// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::validator_api;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorClientError {
    #[error("There was an issue with the validator api request - {source}")]
    ValidatorAPIError {
        #[from]
        source: validator_api::error::ValidatorAPIError,
    },

    #[error("One of the provided URLs was malformed - {0}")]
    MalformedUrlProvided(#[from] url::ParseError),

    #[cfg(feature = "nymd-client")]
    #[error("There was an issue with the Nymd client - {0}")]
    NymdError(#[from] crate::nymd::error::NymdError),

    #[error("No validator API url has been provided")]
    NoAPIUrlAvailable,
}
