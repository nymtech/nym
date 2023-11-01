// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to serialize json data: {source}")]
    JsonSerializationFailure {
        #[from]
        source: serde_json::Error,
    },

    #[error(transparent)]
    WireguardError {
        #[from]
        source: nym_wireguard_types::Error,
    },
}
