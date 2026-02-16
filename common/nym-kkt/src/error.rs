// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::KKTStatus;
use nym_kkt_ciphersuite::error::KKTCiphersuiteError;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KKTError {
    #[error(transparent)]
    CiphersuiteDecodingError(#[from] KKTCiphersuiteError),

    #[error(transparent)]
    MaskedByteError(#[from] MaskedByteError),

    #[error("KEM mapping failure: {}", info)]
    KEMMapping { info: &'static str },

    #[error("Insecure Encapsulation Key Hash Length")]
    InsecureHashLen,

    #[error("KKT Frame Decoding Error: {}", info)]
    FrameDecodingError { info: String },

    #[error("KKT Frame Encoding Error: {}", info)]
    FrameEncodingError { info: String },

    #[error("KKT Incompatibility Error: {}", info)]
    IncompatibilityError { info: &'static str },

    #[error("KKT Responder Flagged Error: {}", status)]
    ResponderFlaggedError { status: KKTStatus },

    #[error("KKT Message Count Limit Reached")]
    MessageCountLimitReached,

    #[error("PSQ KEM Error: {}", info)]
    KEMError { info: &'static str },

    #[error("Local Function Input Error: {}", info)]
    FunctionInputError { info: &'static str },

    #[error("{}", info)]
    X25519Error { info: &'static str },

    #[error("{}", info)]
    AEADError { info: &'static str },

    #[error("{}", info)]
    DecodingError { info: &'static str },

    #[error("{}", info)]
    UnsupportedAlgorithm { info: &'static str },

    #[error("Generic libcrux error")]
    LibcruxError,

    #[error("failed to derive shared secret: {inner:?}")]
    SharedSecretDerivationFailure {
        inner: libcrux_psq::handshake::HandshakeError,
    },

    #[error("the received encapsulation key hash does not match the expected value")]
    MismatchedKEMHash,
}

impl KKTError {
    pub fn shared_secret_derivation_failure(inner: libcrux_psq::handshake::HandshakeError) -> Self {
        KKTError::SharedSecretDerivationFailure { inner }
    }
}

#[derive(Error, Debug)]
pub enum MaskedByteError {
    #[error(
        "Invalid Masked Byte Length: Expected({}), Actual({}).",
        expected,
        actual
    )]
    InvalidLength { expected: usize, actual: usize },
    #[error("Failed to Unmask Byte.")]
    Failure,
}

impl From<libcrux_kem::Error> for KKTError {
    fn from(err: libcrux_kem::Error) -> Self {
        match err {
            libcrux_kem::Error::EcDhError(_) => KKTError::KEMError { info: "ECDH Error" },
            libcrux_kem::Error::KeyGen => KKTError::KEMError {
                info: "Key Generation Error",
            },
            libcrux_kem::Error::Encapsulate => KKTError::KEMError {
                info: "Encapsulation Error",
            },
            libcrux_kem::Error::Decapsulate => KKTError::KEMError {
                info: "Decapsulation Error",
            },
            libcrux_kem::Error::UnsupportedAlgorithm => KKTError::KEMError {
                info: "libcrux Unsupported Algorithm",
            },
            libcrux_kem::Error::InvalidPrivateKey => KKTError::KEMError {
                info: "Invalid Private Key",
            },

            libcrux_kem::Error::InvalidPublicKey => KKTError::KEMError {
                info: "Invalid Public Key",
            },
            libcrux_kem::Error::InvalidCiphertext => KKTError::KEMError {
                info: "Invalid Ciphertext",
            },
        }
    }
}
impl From<libcrux_ecdh::Error> for KKTError {
    fn from(err: libcrux_ecdh::Error) -> Self {
        match err {
            libcrux_ecdh::Error::InvalidPoint => KKTError::KEMError {
                info: "Invalid Remote Public Key",
            },
            _ => KKTError::LibcruxError,
        }
    }
}
impl From<libcrux_chacha20poly1305::AeadError> for KKTError {
    fn from(err: libcrux_chacha20poly1305::AeadError) -> Self {
        KKTError::KEMError {
            info: match err {
                libcrux_chacha20poly1305::AeadError::PlaintextTooLarge => {
                    "Plaintext is longer than u32::MAX"
                }
                libcrux_chacha20poly1305::AeadError::CiphertextTooLarge => {
                    "Ciphertext is longer than u32::MAX"
                }
                libcrux_chacha20poly1305::AeadError::AadTooLarge => "Aad is longer than u32::MAX",
                libcrux_chacha20poly1305::AeadError::CiphertextTooShort => {
                    "The provided destination ciphertext does not fit the ciphertext and tag"
                }
                libcrux_chacha20poly1305::AeadError::PlaintextTooShort => {
                    "The provided destination plaintext is too short to fit the decrypted plaintext"
                }
                libcrux_chacha20poly1305::AeadError::InvalidCiphertext => {
                    "The ciphertext is not a valid encryption under the given key and nonce."
                }
            },
        }
    }
}
