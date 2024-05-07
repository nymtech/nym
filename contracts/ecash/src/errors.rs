// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Received multiple coin types")]
    MultipleDenoms,

    #[error("No coin was sent for voucher")]
    NoCoin,

    #[error("Wrong coin denomination, you must send {mix_denom}")]
    WrongDenom { mix_denom: String },

    #[error("Wrong amount for deposit, you must send {amount}")]
    WrongAmount { amount: u128 },

    #[error("There aren't enough funds in the contract")]
    NotEnoughFunds,

    #[error("Credential already spent or in process of spending")]
    DuplicateBlindedSerialNumber,

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("Proposal error - {0}")]
    ProposalError(String),

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("Unauthorized")]
    Unauthorized,
}
