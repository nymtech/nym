// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NymPoolContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },
    /*
    #[error(transparent)]
    Admin(#[from] AdminError),
     */
    #[error("{source}")]
    StdErr {
        #[from]
        source: cosmwasm_std::StdError,
    },
}
