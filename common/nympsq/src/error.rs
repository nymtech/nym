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

    #[error("encountered a Serialization/Deserialization error")]
    SerializationError,

    #[error("encountered an IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Incorrect state")]
    IncorrectStateError,

    #[error("Handshake did not complete")]
    HandshakeError,

    #[error("Unknown PSQ version")]
    UnknownVersion,
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

impl From<tls_codec::Error> for PSQError {
    fn from(err: tls_codec::Error) -> Self {
        match err {
            err => PSQError::SerializationError,
        }
    }
}
