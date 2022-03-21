// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrimitivesError {
    #[error("{source}")]
    CosmwasmError {
        #[from]
        source: cosmwasm_std::StdError,
    },
    #[error("{source}")]
    CosmrsError {
        #[from]
        source: cosmrs::ErrorReport,
    },
}
