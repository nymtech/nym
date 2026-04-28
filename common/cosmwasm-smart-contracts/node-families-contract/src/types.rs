// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NodeFamiliesContractError {
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}
