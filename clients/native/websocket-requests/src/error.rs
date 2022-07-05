// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt;

// no need to go fancy here like we've done in other places.
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind.as_str(), self.message)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error {
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Error { kind, message }
    }
}

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum ErrorKind {
    /// The received request contained no data.
    EmptyRequest = 0x01,

    /// The received request did not contain enough data to be fully parsed.
    TooShortRequest = 0x02,

    /// The received request tag is not defined.
    UnknownRequest = 0x03,

    /// The received request is malformed.
    MalformedRequest = 0x04,

    // that's an arbitrary division but let's keep 1-127 (hex 0x01 - 0x7F) values request-specific
    // and 128-254 (hex 0x80 - 0xFE) for responses
    /// The received response contained no data.
    EmptyResponse = 0x80,

    /// The received response did not contain enough data to be fully parsed.
    TooShortResponse = 0x81,

    /// The received response tag is not defined.
    UnknownResponse = 0x82,

    /// The received response is malformed.
    MalformedResponse = 0x83,

    /// The error is due to something else.
    Other = 0xFF,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::EmptyRequest => "received request contained no data",
            ErrorKind::TooShortRequest => "received request did not contain enough data",
            ErrorKind::UnknownRequest => "unknown request type",
            ErrorKind::MalformedRequest => "malformed request",

            ErrorKind::EmptyResponse => "received response contained no data",
            ErrorKind::TooShortResponse => "received response did not contain enough data",
            ErrorKind::UnknownResponse => "unknown response type",
            ErrorKind::MalformedResponse => "malformed response",

            ErrorKind::Other => "other",
        }
    }
}
