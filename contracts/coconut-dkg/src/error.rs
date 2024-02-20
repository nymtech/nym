// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, StdError};
use cw_controllers::AdminError;
use nym_coconut_dkg_common::dealing::MAX_DEALING_CHUNKS;
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, EpochId};
use thiserror::Error;

/// Custom errors for contract failure conditions.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("Dkg hasn't been initialised yet")]
    WaitingInitialisation,

    #[error("Dkg has already been initialised")]
    AlreadyInitialised,

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
        "Requested action needs state to be {expected_state}, currently in state {current_state}"
    )]
    IncorrectEpochState {
        current_state: String,
        expected_state: String,
    },

    #[error("This sender is not a dealer for the current epoch")]
    NotADealer,

    #[error("This sender is not a dealer for the current resharing epoch")]
    NotAnInitialDealer,

    #[error("Dealer {dealer} has already committed dealing chunk for epoch {epoch_id} with dealing index {dealing_index} and chunk index {chunk_index} at height {block_height}")]
    DealingChunkAlreadyCommitted {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
        block_height: u64,
    },

    #[error("dealer {dealer} tried to commit chunk {chunk_index} of dealing {dealing_index} for epoch {epoch_id}, but it hasn't been declared in the prior metadata")]
    DealingChunkNotInMetadata {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    },

    #[error("dealer {dealer} has attempted to commit dealing chunk for epoch {epoch_id} with dealing index {index} while the key size is set to {key_size}")]
    DealingOutOfRange {
        epoch_id: EpochId,
        dealer: Addr,
        index: DealingIndex,
        key_size: u32,
    },

    #[error("dealer {dealer} has attempted to commit dealing metadata for epoch {epoch_id} for dealing index {dealing_index} with {chunks} chunks while at most {} chunks are allowed", MAX_DEALING_CHUNKS)]
    TooFragmentedMetadata {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
        chunks: usize,
    },

    #[error("the declared chunk split for epoch {epoch_id} from dealer {dealer} for dealing index {dealing_index} is uneven. first chunk has size of {first_chunk_size} while chunk at index {chunk_index} has {size}")]
    UnevenChunkSplit {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
        first_chunk_size: u64,
        size: u64,
    },

    #[error("the received chunk for epoch {epoch_id} from dealer {dealer} at dealing index {dealing_index} at chunk index {chunk_index} has inconsistent length. the metadata contains length of {metadata_length} while the received data is {received} bytes long")]
    InconsistentChunkLength {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
        metadata_length: u64,
        received: u64,
    },

    #[error("dealer {dealer} has attempted to commit dealing metadata for epoch {epoch_id} for dealing index {dealing_index} zero chunks")]
    EmptyMetadata {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
    },

    #[error("metadata for dealing for epoch {epoch_id} from {dealer} at index {dealing_index} does not exist")]
    UnavailableDealingMetadata {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
    },

    #[error("metadata for dealing for epoch {epoch_id} from {dealer} at index {dealing_index} already exists")]
    MetadataAlreadyExists {
        epoch_id: EpochId,
        dealer: Addr,
        dealing_index: DealingIndex,
    },

    #[error("This dealer has already committed {commitment}")]
    AlreadyCommitted { commitment: String },

    #[error("No verification key committed for owner {owner}")]
    NoCommitForOwner { owner: String },

    #[error("failed to parse {value} into a valid SemVer version: {error_message}")]
    SemVerFailure {
        value: String,
        error_message: String,
    },
}
