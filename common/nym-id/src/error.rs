// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use thiserror::Error;
use time::Date;

#[derive(Debug, Error)]
pub enum NymIdError {
    #[error("failed to deserialize provided full ticketbook: {source}")]
    FullTicketbookDeserializationFailure { source: nym_credentials::Error },

    #[error("failed to deserialize provided ticketbook: {source}")]
    TicketbookDeserializationFailure { source: nym_credentials::Error },

    #[error("failed to deserialize provided expiration date signatures: {source}")]
    ExpirationDateSignaturesDeserializationFailure { source: nym_credentials::Error },

    #[error("failed to deserialize provided coin index signatures: {source}")]
    CoinIndexSignaturesDeserializationFailure { source: nym_credentials::Error },

    #[error("failed to deserialize provided verification key: {source}")]
    VerificationKeyDeserializationFailure { source: nym_credentials::Error },

    #[error("attempted to import an expired credential (it expired on {expiration})")]
    ExpiredCredentialImport { expiration: Date },

    #[error("could not import ticketbook expiring at {date} since we do not have corresponding expiration date signatures")]
    MissingExpirationDateSignatures { date: Date },

    #[error("could not import ticketbook for epoch {epoch_id} since we do not have corresponding coin index signatures")]
    MissingCoinIndexSignatures { epoch_id: u64 },

    #[error("could not import ticketbook for epoch {epoch_id} since we do not have corresponding master verification key")]
    MissingMasterVerificationKey { epoch_id: u64 },

    #[error("failed to store credential in the provided store: {source}")]
    StorageError {
        source: Box<dyn Error + Send + Sync>,
    },
}
