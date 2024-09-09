// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use time::Date;

use crate::ecash::error::EcashTicketError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the provided bandwidth credential has already been spent before at this gateway")]
    BandwidthCredentialAlreadySpent,

    #[error(transparent)]
    EcashFailure(EcashTicketError),

    #[error(
        "the provided credential has an invalid spending date. got {got} but expected {expected}"
    )]
    InvalidCredentialSpendingDate { got: Date, expected: Date },

    #[error("the current multisig contract is not using 'AbsolutePercentage' threshold!")]
    InvalidMultisigThreshold,

    #[error(
        "the received payment contained more than a single ticket. that's currently not supported"
    )]
    MultipleTickets,

    #[error("Nyxd Error - {0}")]
    NyxdError(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("This gateway is only accepting coconut credentials for bandwidth")]
    OnlyCoconutCredentials,

    #[error("insufficient bandwidth available to process the request. required: {required}B, available: {available}B")]
    OutOfBandwidth { required: i64, available: i64 },

    #[error("Internal gateway storage error")]
    StorageError(#[from] nym_gateway_storage::error::StorageError),

    #[error("{0}")]
    UnknownTicketType(#[from] nym_credentials_interface::UnknownTicketType),
}

impl From<EcashTicketError> for Error {
    fn from(err: EcashTicketError) -> Self {
        // don't expose storage issue details to the user
        if let EcashTicketError::InternalStorageFailure { source } = err {
            Error::StorageError(source)
        } else {
            Error::EcashFailure(err)
        }
    }
}
