// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::StdError;
use thiserror::Error;

/// Custom errors for contract failure conditions.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("This potential ephemera peer is not in the ephemera group")]
    Unauthorized,

    #[error("This sender is already registered")]
    AlreadyRegistered,
}
