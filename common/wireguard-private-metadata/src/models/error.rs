// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::Version;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error("trying to deserialize from version {source_version} into {target_version}")]
    InvalidVersion {
        source_version: Version,
        target_version: Version,
    },
}
