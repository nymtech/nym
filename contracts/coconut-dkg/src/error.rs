// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, StdError, VerificationError};
use thiserror::Error;

/// Custom errors for contract failure conditions.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("No coin was sent for the deposit, you must send {denom}")]
    NoDepositFound { denom: String },

    #[error("Received multiple coin types")]
    MultipleDenoms,

    #[error("Wrong coin denomination, you must send {denom}")]
    WrongDenom { denom: String },

    #[error("Not enough funds sent for deposit. (received {received}, minimum {minimum})")]
    InsufficientDeposit { received: u128, minimum: u128 },

    #[error("Failed to perform ed25519 signature verification - {0}. This dealer will be temporarily blacklisted now.")]
    Ed25519VerificationError(#[from] VerificationError),

    #[error("Provided ed25519 signature did not verify correctly. This dealer will be temporarily blacklisted now.")]
    InvalidEd25519Signature,

    #[error("This potential dealer is not in the coconut signer group")]
    Unauthorized,

    #[error("This sender is already a dealer for the epoch")]
    AlreadyADealer,

    #[error("Epoch hasn't been correctly initialised!")]
    EpochNotInitialised,

    // we should never ever see this error (famous last words in programming), therefore, I'd want to
    // explicitly declare it so that when we ultimate do see it, it's gonna be more informative over "normal" panic
    #[error("Somehow our validated address {address} is not using correct bech32 encoding")]
    InvalidValidatedAddress { address: Addr },

    #[error("This sender is not a dealer for the current epoch")]
    NotADealer,

    #[error("This dealer has already commited dealing for this epoch")]
    AlreadyCommitted,
}
