// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{EpochId, NodeId};
use cosmwasm_std::Addr;
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

    #[error("attempted to submit performance data for epoch {epoch_id} and node {node_id} whilst last submitted was {last_epoch_id} for node {last_node_id}")]
    StalePerformanceSubmission {
        epoch_id: EpochId,
        node_id: NodeId,
        last_epoch_id: EpochId,
        last_node_id: NodeId,
    },

    #[error("the batch performance data has not been sorted")]
    UnsortedBatchSubmission,
}
