// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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
}
