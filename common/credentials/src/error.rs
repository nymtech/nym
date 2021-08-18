// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_interface::CoconutError;
use thiserror::Error;
use validator_client::ValidatorClientError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("The detailed description is yet to be determined")]
    BandwidthCredentialError,

    #[error("Could not contact any validator")]
    NoValidatorsAvailable,

    #[error("Run into a coconut error - {0}")]
    CoconutError(#[from] CoconutError),

    #[error("Run into a validato client error - {0}")]
    ValidatorClientError(#[from] ValidatorClientError),
}
