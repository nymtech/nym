// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::Version;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error("trying to deserialize from version {source_version:?} into {target_version:?}")]
    InvalidVersion {
        source_version: Version,
        target_version: Version,
    },

    #[error(
        "trying to deserialize from query type {source_query_type} query type {target_query_type}"
    )]
    InvalidQueryType {
        source_query_type: String,
        target_query_type: String,
    },

    #[error("update not possible from {from:?} to {to:?}")]
    UpdateNotPossible { from: Version, to: Version },

    #[error("downgrade not possible from {from:?} to {to:?}")]
    DowngradeNotPossible { from: Version, to: Version },
}
