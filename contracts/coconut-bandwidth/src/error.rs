// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;

use config::defaults::DENOM;

/// Custom errors for contract failure conditions.
///
/// Add any other custom errors you like here.
/// Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Received multiple coin types")]
    MultipleDenoms,

    #[error("No coin was sent for voucher")]
    NoCoin,

    #[error("Wrong coin denomination, you must send {}", DENOM)]
    WrongDenom,

    #[error("There aren't enough funds in the contract")]
    NotEnoughFunds,

    #[error("{0}")]
    Admin(#[from] AdminError),
}
