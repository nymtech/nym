// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{EpochId, NodeId};
use cosmwasm_std::{Addr, Decimal};
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NymPerformanceContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),

    #[error("{address} is already an authorised network monitor")]
    AlreadyAuthorised { address: Addr },

    #[error("{address} is not an authorised network monitor")]
    NotAuthorised { address: Addr },

    #[error(
        "attempted to submit performance data for epoch {epoch_id} and node {node_id} whilst last submitted was {last_epoch_id} for node {last_node_id}"
    )]
    StalePerformanceSubmission {
        epoch_id: EpochId,
        node_id: NodeId,
        last_epoch_id: EpochId,
        last_node_id: NodeId,
    },

    #[error("the batch performance data has not been sorted")]
    UnsortedBatchSubmission,

    #[error("node {node_id} does not appear to be bonded")]
    NodeNotBonded { node_id: NodeId },

    #[error(
        "the current mixnet epoch is {current_epoch_id}, so data for epoch {epoch_id} cannot be submitted"
    )]
    EpochNotCurrent {
        epoch_id: EpochId,
        current_epoch_id: EpochId,
    },

    #[error("the submission for node {node_id} carries no measurements")]
    EmptyNodeSubmission { node_id: NodeId },

    #[error("at least one routing kind must carry a non-zero weight")]
    EmptyWeights,

    #[error("the weights sum to {total} rather than 1")]
    WeightsDoNotSumToOne { total: Decimal },
}
