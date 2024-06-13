// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Coin, StdError};
use cw_controllers::AdminError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum EcashContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Invalid deposit")]
    InvalidDeposit(#[from] PaymentError),

    #[error("received wrong amount for deposit. got: {received}. required: {amount}")]
    WrongAmount { received: u128, amount: u128 },

    #[error("There aren't enough funds in the contract")]
    NotEnoughFunds,

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("could not find proposal id inside the multisig reply SubMsg")]
    MissingProposalId,

    // realistically this should NEVER be thrown
    #[error("the proposal id returned by the multisig contract could not be parsed into an u64")]
    MalformedProposalId,

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Failed to parse {value} into a valid SemVer version: {error_message}")]
    SemVerFailure {
        value: String,
        error_message: String,
    },

    #[error("received an invalid reply id: {id}. it does not correspond to any sent SubMsg")]
    InvalidReplyId { id: u64 },

    #[error("reached the maximum of 255 different deposit types")]
    MaximumDepositTypesReached,

    #[error("compressed deposit info {typ} does not corresponds to any known type")]
    UnknownCompressedDepositInfoType { typ: u8 },

    #[error("deposit info {typ} does not corresponds to any previously seen type")]
    UnknownDepositInfoType { typ: String },

    #[error("the provided ed25519 identity was malformed")]
    MalformedEd25519Identity,

    #[error("the required deposit amount has changed since the contract was created! This was not expected! It used to be {at_init} but it's {current} now! Please let the developers know ASAP!")]
    DepositAmountChanged { at_init: Coin, current: Coin },
}
