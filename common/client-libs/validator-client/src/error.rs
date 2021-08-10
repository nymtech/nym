// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::validator_api;
use serde::Deserialize;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorClientError {
    #[error("There was an issue with the REST request - {source}")]
    ReqwestClientError {
        #[from]
        source: reqwest::Error,
    },

    #[error("There was an issue with the validator-api request - {source}")]
    ValidatorAPIError {
        #[from]
        source: validator_api::error::ValidatorAPIClientError,
    },

    #[error("An IO error has occured: {source}")]
    IoError {
        #[from]
        source: io::Error,
    },

    #[error("There was an issue with the validator client - {0}")]
    ValidatorError(String),

    #[cfg(feature = "nymd-client")]
    #[error("There was an issue with the Nymd clientn - {0}")]
    NymdError(#[from] crate::nymd::error::NymdError),
}

// this is the case of message like
/*
{
  "code": 12,
  "message": "Not Implemented",
  "details": [
  ]
}
 */
// I didn't manage to find where it exactly originates, nor what the correct types should be
// so all of those are some educated guesses

#[derive(Error, Debug, Deserialize)]
#[error("code: {code} - {message}")]
pub(super) struct CodedError {
    code: u32,
    message: String,
    details: Vec<(String, String)>,
}

#[derive(Error, Deserialize, Debug)]
#[error("{error}")]
pub(super) struct SmartQueryError {
    pub(super) error: String,
}
