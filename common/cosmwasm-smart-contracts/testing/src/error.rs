// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MockingError {
    #[error(transparent)]
    StdError {
        #[from]
        source: StdError,
    },

    #[error("attempted to add another contract mock that has the same address as an existing one - {address}")]
    DuplicateContractAddress { address: Addr },

    #[error("attempted to use a contract that doesn't exist - {address}")]
    NonExistentContract { address: Addr },

    #[error("contract execution failed with error: {error}. We called {contract} with {message}")]
    ContractExecutionError {
        message: String,
        contract: Addr,
        error: String,
    },

    #[error("contract query failed with error: {error}. We called {contract} with {message}")]
    ContractQueryError {
        message: String,
        contract: Addr,
        error: String,
    },
}
