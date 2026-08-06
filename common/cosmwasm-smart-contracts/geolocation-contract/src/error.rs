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

    /// The payload's `content` exceeded the configured maximum size.
    #[error("payload content is {len} bytes, exceeding the {max} byte limit")]
    PayloadTooLarge { len: usize, max: u32 },

    /// A payload was decoded against a version it was not written under. The contract never
    /// raises this, since it stores payloads opaquely; it is for producers and consumers.
    #[error("expected a version {expected} payload, got version {got}")]
    UnexpectedPayloadVersion { expected: u8, got: u8 },

    /// A payload's `content` did not decode under its own version's format.
    #[error("malformed payload content: {0}")]
    MalformedPayload(String),

    /// A storage key's trailing source component could not be decoded.
    #[error("malformed source encoding: {0}")]
    InvalidSourceEncoding(String),

    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}
