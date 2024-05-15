// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::StdError;
use cw_controllers::AdminError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Invalid deposit")]
    InvalidDeposit(#[from] PaymentError),

    #[error("Wrong amount for deposit, you must send {amount}")]
    WrongAmount { amount: u128 },

    #[error("There aren't enough funds in the contract")]
    NotEnoughFunds,

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("Proposal error - {0}")]
    ProposalError(String),

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("Unauthorized")]
    Unauthorized,
}
