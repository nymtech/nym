// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::NymApiStorageError;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::{EpochState, NodeId};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::ValidatorClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RewardingError {
    #[error("Our account ({our_address}) is not permitted to update rewarded set and perform rewarding. The allowed address is {allowed_address}")]
    Unauthorised {
        our_address: AccountId,
        allowed_address: AccountId,
    },

    #[error("the current epoch is in the wrong state ({current_state}) to perform the requested operation: {operation}")]
    InvalidEpochState {
        current_state: EpochState,
        operation: String,
    },

    #[error("it seems the current epoch is in mid-rewarding state (last rewarded is {last_rewarded}). With our current nym-api this shouldn't have been possible. Manual intervention is required.")]
    MidNodeRewarding { last_rewarded: NodeId },

    #[error("it seems the current epoch is in mid-role assignment state (next role to assign is {next}). With our current nym-api this shouldn't have been possible. Manual intervention is required.")]
    MidRoleAssignment { next: Role },

    // #[error("There were no mixnodes to reward (network is dead)")]
    // NoMixnodesToReward,
    #[error("Failed to execute the smart contract - {0}")]
    ContractExecutionFailure(NyxdError),

    // The inner error should be modified at some point...
    #[error("We run into storage issues - {0}")]
    StorageError(NymApiStorageError),

    #[error("Failed to query the smart contract - {0}")]
    ValidatorClientError(ValidatorClientError),

    #[error("Error downcasting u128 -> u64")]
    DowncastingError {
        #[from]
        source: std::num::TryFromIntError,
    },

    #[error("{source}")]
    WeightedError {
        #[from]
        source: rand::distributions::WeightedError,
    },

    #[error("could not obtain the current interval rewarding parameters")]
    RewardingParamsRetrievalFailure,

    #[error("{0}")]
    GenericError(#[from] anyhow::Error),
}

impl From<NyxdError> for RewardingError {
    fn from(err: NyxdError) -> Self {
        RewardingError::ContractExecutionFailure(err)
    }
}

impl From<NymApiStorageError> for RewardingError {
    fn from(err: NymApiStorageError) -> Self {
        RewardingError::StorageError(err)
    }
}

impl From<ValidatorClientError> for RewardingError {
    fn from(err: ValidatorClientError) -> Self {
        RewardingError::ValidatorClientError(err)
    }
}
