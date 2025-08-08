// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_noise_keys::NoiseVersion;
use nympsq::error::PSQError;
use snow::Error;
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoiseError {
    #[error("encountered a Noise decryption error")]
    DecryptionError,

    #[error("encountered a Noise Protocol error: {0}")]
    ProtocolError(Error),

    #[error("encountered an IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Incorrect state")]
    IncorrectStateError,

    #[error("Handshake did not complete")]
    HandshakeError,

    #[error("unknown noise version (encoded value: {encoded})")]
    UnknownVersion { encoded: u8 },

    #[error("unknown noise pattern (encoded value: {encoded})")]
    UnknownPattern { encoded: u8 },

    #[error("unknown noise message type (encoded value: {encoded})")]
    UnknownMessageType { encoded: u8 },

    #[error("failed to generate psk for requested version {noise_version}")]
    PskGenerationFailure { noise_version: u8 },

    #[error("noise initiator attempted to use version v{noise_version} of the protocol - we don't know how to handle it")]
    UnknownVersionHandshake { noise_version: u8 },

    #[error("noise initiator attempted to use an unexpected noise pattern. we're configured for {configured} while it requested {received}")]
    UnexpectedNoisePattern {
        configured: &'static str,
        received: &'static str,
    },

    #[error("handshake version has unexpectedly changed. initial was {initial:?} and received {received:?}")]
    UnexpectedHandshakeVersion {
        initial: NoiseVersion,
        received: NoiseVersion,
    },

    #[error("data packet version has unexpectedly changed. initial was {initial:?} and received {received:?}")]
    UnexpectedDataVersion {
        initial: NoiseVersion,
        received: NoiseVersion,
    },

    #[error("received a non-handshake message during noise handshake")]
    NonHandshakeMessageReceived,

    #[error("received a non-data message post noise handshake")]
    NonDataMessageReceived,

    #[error("handshake message exceeded maximum size (got {size} bytes)")]
    HandshakeTooBig { size: usize },

    #[error("noise message exceeded maximum size (got {size} bytes)")]
    DataTooBig { size: usize },

    #[error("Handshake timeout")]
    HandshakeTimeout(#[from] tokio::time::error::Elapsed),

    #[error("PSQ Error")]
    PSQError(PSQError),
}

impl NoiseError {
    pub(crate) fn naive_to_io_error(self) -> std::io::Error {
        match self {
            NoiseError::IoError(err) => err,
            other => std::io::Error::other(other),
        }
    }
}

impl From<PSQError> for NoiseError {
    fn from(err: PSQError) -> Self {
        NoiseError::PSQError(err)
    }
}

impl From<libcrux_kem::Error> for NoiseError {
    fn from(err: libcrux_kem::Error) -> Self {
        NoiseError::PSQError(err.into())
    }
}

impl From<Error> for NoiseError {
    fn from(err: Error) -> Self {
        match err {
            Error::Decrypt => NoiseError::DecryptionError,
            err => NoiseError::ProtocolError(err),
        }
    }
}
