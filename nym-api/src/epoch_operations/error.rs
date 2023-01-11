// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::NymApiStorageError;
use thiserror::Error;
use validator_client::nyxd::error::NyxdError;
use validator_client::nyxd::AccountId;
use validator_client::ValidatorClientError;

#[derive(Debug, Error)]
pub enum RewardingError {
    #[error("Our account ({our_address}) is not permitted to update rewarded set and perform rewarding. The allowed address is {allowed_address}")]
    Unauthorised {
        our_address: AccountId,
        allowed_address: AccountId,
    },

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
