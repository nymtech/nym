// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::storage::error::StorageError;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EcashTicketError {
    // TODO: this should be more granual
    #[error(transparent)]
    ApiFailure(#[from] EcashApiError),

    #[error(transparent)]
    CredentialError(#[from] nym_credentials::error::Error),

    #[error("the provided ticket failed to get verified")]
    MalformedTicket,

    #[error("failed to verify provided ticket due to invalid expiration date signatures")]
    MalformedTicketInvalidDateSignatures,

    #[error("provided payinfo's public key does not match provider's")]
    InvalidPayInfoPublicKey,

    #[error("provided payinfo's timestamp is invalid")]
    InvalidPayInfoTimestamp,

    #[error("received payinfo is a duplicate")]
    DuplicatePayInfo,

    #[error("could not handle the ecash ticket due to internal storage failure: {source}")]
    InternalStorageFailure {
        #[from]
        source: StorageError,
    },

    #[error("failed to create ticket redemption proposal: {source}")]
    RedemptionProposalCreationFailure {
        #[source]
        source: NyxdError,
    },

    #[error("failed to execute ticket redemption proposal {proposal_id}: {source}")]
    RedemptionProposalExecutionFailure {
        proposal_id: u64,

        #[source]
        source: NyxdError,
    },

    #[error("failed to parse out the redemption proposal id: {source}")]
    ProposalIdParsingFailure {
        #[source]
        source: NyxdError,
    },

    #[error("failed to query the nyx chain: {source}")]
    ChainQueryFailure {
        #[source]
        source: NyxdError,
    },

    #[error("Not enough nym API endpoints provided. Needed {needed}, received {received}")]
    NotEnoughNymAPIs { received: usize, needed: usize },

    #[error("the DKG contract is unavailable")]
    UnavailableDkgContract,

    #[error("the DKG threshold value for epoch {epoch_id} is currently unavailable. we're probably mid-epoch transition")]
    DKGThresholdUnavailable { epoch_id: EpochId },

    #[error("could not create redemption proposal as we have tickets pending full verification")]
    PendingTickets,
}

impl EcashTicketError {
    pub fn chain_query_failure(source: NyxdError) -> EcashTicketError {
        EcashTicketError::ChainQueryFailure { source }
    }
}
