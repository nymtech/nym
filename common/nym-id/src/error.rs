// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use thiserror::Error;
use time::Date;

#[derive(Debug, Error)]
pub enum NymIdError {
    #[error("failed to deserialize provided credential: {source}")]
    CredentialDeserializationFailure { source: nym_credentials::Error },

    #[error("attempted to import an expired credential (it expired on {expiration})")]
    ExpiredCredentialImport { expiration: Date },

    #[error("failed to store credential in the provided store: {source}")]
    StorageError {
        source: Box<dyn Error + Send + Sync>,
    },
}
