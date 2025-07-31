// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ProposalId;
use cosmwasm_std::Addr;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NymOfflineSignersContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error("can't require more than 100% of signers for quorum")]
    RequiredQuorumBiggerThanOne,

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),

    #[error("{address} is not a member of the authorised DKG group")]
    NotGroupMember { address: Addr },

    #[error("{address} is already marked as offline")]
    AlreadyOffline { address: Addr },

    #[error("{address} is not marked as offline nor in the process of being voted on")]
    NotOffline { address: Addr },

    #[error("{voter} has already voted to mark {target} as offline in proposal {proposal}")]
    AlreadyVoted {
        voter: Addr,
        proposal: ProposalId,
        target: Addr,
    },

    #[error("{address} has only recently came back online")]
    RecentlyCameOnline { address: Addr },

    #[error("{address} has only recently came offline")]
    RecentlyCameOffline { address: Addr },
}
