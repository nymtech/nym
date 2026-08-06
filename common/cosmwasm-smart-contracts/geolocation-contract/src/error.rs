// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::SubjectClass;
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GeolocationContractError {
    /// A subject id whose byte width did not match the fixed width its class requires.
    /// The width has to be constant within a class, otherwise the storage key's length
    /// prefix varies and entries stop ordering by id content.
    #[error("subject id for class {class} must be {expected} bytes, got {actual}")]
    InvalidSubjectId {
        class: SubjectClass,
        expected: usize,
        actual: usize,
    },

    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}
