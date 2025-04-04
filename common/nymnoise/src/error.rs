// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use snow::Error;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoiseError {
    #[error("encountered a Noise decryption error")]
    DecryptionError,

    #[error("encountered a Noise Protocol error - {0}")]
    ProtocolError(Error),

    #[error("encountered an IO error - {0}")]
    IoError(#[from] io::Error),

    #[error("Incorrect state")]
    IncorrectStateError,

    #[error("Handshake did not complete")]
    HandshakeError,

    #[error("Unknown noise version")]
    UnknownVersion,
}

impl From<Error> for NoiseError {
    fn from(err: Error) -> Self {
        match err {
            Error::Decrypt => NoiseError::DecryptionError,
            err => NoiseError::ProtocolError(err),
        }
    }
}
