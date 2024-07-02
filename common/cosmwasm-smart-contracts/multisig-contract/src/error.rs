// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::StdError;
use cw3::DepositError;
use cw_utils::{PaymentError, ThresholdError};

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Threshold(#[from] ThresholdError),

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Proposal is not open")]
    NotOpen {},

    #[error("Proposal voting period has expired")]
    Expired {},

    #[error("Proposal must expire before you can close it")]
    NotExpired {},

    #[error("Wrong expiration option")]
    WrongExpiration {},

    #[error("Already voted on this proposal")]
    AlreadyVoted {},

    #[error("Proposal must have passed and not yet been executed")]
    WrongExecuteStatus {},

    #[error("Cannot close completed or passed proposals")]
    WrongCloseStatus {},

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    Deposit(#[from] DepositError),

    #[error("the provided redemption digest does not have valid base58 encoding or is not 32 bytes long")]
    MalformedRedemptionDigest,

    #[error("the provided redemption proposal data is malformed and can't be decoded")]
    MalformedRedemptionProposalData,
}
