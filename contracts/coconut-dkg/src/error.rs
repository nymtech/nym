// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, StdError};
use cw_controllers::AdminError;
use nym_coconut_dkg_common::types::{DealingIndex, EpochId};
use thiserror::Error;

/// Custom errors for contract failure conditions.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("Group contract invalid address '{addr}'")]
    InvalidGroup { addr: String },

    #[error("This potential dealer is not in the coconut signer group")]
    Unauthorized,

    #[error("This sender is already a dealer for the epoch")]
    AlreadyADealer,

    #[error("Too soon to advance epoch state. {0} more seconds until it can be advanced")]
    EarlyEpochStateAdvancement(u64),

    #[error("Epoch hasn't been correctly initialised!")]
    EpochNotInitialised,

    #[error(
        "Requested action needs state to be {expected_state}, currently in state {current_state}, "
    )]
    IncorrectEpochState {
        current_state: String,
        expected_state: String,
    },

    #[error("This sender is not a dealer for the current epoch")]
    NotADealer,

    #[error("This sender is not a dealer for the current resharing epoch")]
    NotAnInitialDealer,

    #[error(
        "Dealer {dealer} has already committed dealing for epoch {epoch_id} with index {index}"
    )]
    DealingAlreadyCommitted {
        epoch_id: EpochId,
        dealer: Addr,
        index: DealingIndex,
    },

    #[error(
    "Dealer {dealer} has attempted to commit dealing for epoch {epoch_id} with index {index} while the key size is set to {key_size}"
    )]
    DealingOutOfRange {
        epoch_id: EpochId,
        dealer: Addr,
        index: DealingIndex,
        key_size: u32,
    },

    #[error("This dealer has already committed {commitment}")]
    AlreadyCommitted { commitment: String },

    #[error("No verification key committed for owner {owner}")]
    NoCommitForOwner { owner: String },
}
