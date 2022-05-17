// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;
use thiserror::Error;
use validator_client::nymd::error::NymdError;
use validator_client::ValidatorClientError;

#[derive(Error, Debug)]
pub enum DkgError {
    #[error("Internal error - {0}")]
    Internal(#[from] ::dkg::error::DkgError),

    #[error("{0}")]
    ContractClient(#[from] ValidatorClientError),

    #[error("{0}")]
    NymdClient(#[from] NymdError),

    #[error("Networking error - {0}")]
    Networking(#[from] io::Error),

    #[error("Failed to serialize message - {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("Failed to recover assigned node index")]
    NodeIndexRecoveryError,

    #[error("todo")]
    MalformedDealerDetails,

    #[error("todo")]
    DeserializationError,
}
