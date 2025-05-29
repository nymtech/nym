// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PSQError {
    #[error("encountered a decryption error")]
    DecryptionError,

    #[error("encountered a KEM error")]
    KEMError,

    #[error("encountered a PSQ error")]
    PSQError,

    #[error("encountered an IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Incorrect state")]
    IncorrectStateError,

    #[error("Handshake did not complete")]
    HandshakeError,

    #[error("Unknown noise version")]
    UnknownVersion,

    #[error("Handshake timeout")]
    HandshakeTimeout(#[from] tokio::time::error::Elapsed),
}

impl From<libcrux_kem::Error> for PSQError {
    fn from(err: libcrux_kem::Error) -> Self {
        match err {
            // Error::Decrypt => PSQError::DecryptionError,
            err => PSQError::KEMError,
        }
    }
}
impl From<libcrux_psq::Error> for PSQError {
    fn from(err: libcrux_psq::Error) -> Self {
        match err {
            // Error::Decrypt => PSQError::DecryptionError,
            err => PSQError::PSQError,
        }
    }
}

#[derive(Error, Debug)]
pub enum KKTError {
    #[error("Signature constructor error")]
    SigConstructorError,

    #[error("Signature verification error")]
    SigVerifError,
    // #[error("Protocol did not complete")]
    // ProtocolError,

    // #[error("encountered an IO error: {0}")]
    // IoError(#[from] io::Error),

    // #[error("Handshake timeout")]
    // HandshakeTimeout(#[from] tokio::time::error::Elapsed),
}
