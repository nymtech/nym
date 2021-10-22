// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::StdError;
use thiserror::Error;

/// Custom errors for contract failure conditions.
///
/// Add any other custom errors you like here.
/// Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid size for signature items")]
    InvalidSignatureSize,

    #[error("This payment has already been claimed by someone")]
    PaymentAlreadyClaimed,

    #[error("Error parsing signature")]
    ParseSignatureError,

    #[error("The payment is not properly signed")]
    BadSignature,
}
