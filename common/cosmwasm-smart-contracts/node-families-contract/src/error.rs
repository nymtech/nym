// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cw_controllers::AdminError;
use thiserror::Error;

/// Errors returned from any entry point of the node families contract.
#[derive(Error, Debug, PartialEq)]
pub enum NodeFamiliesContractError {
    /// Returned from `migrate` when the on-chain state cannot be brought forward
    /// to the current contract version (e.g. unsupported source version, malformed
    /// stored data).
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    /// Wraps errors raised by `cw-controllers::Admin` (e.g. caller is not admin).
    #[error(transparent)]
    Admin(#[from] AdminError),

    /// Wraps any underlying `cosmwasm_std::StdError` (storage, serialization, etc.).
    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}
