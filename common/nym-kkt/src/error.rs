// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use crate::context::KKTStatus;

#[derive(Error, Debug)]
pub enum KKTError {
    #[error("Signature constructor error")]
    SigConstructorError,
    #[error("Signature verification error")]
    SigVerifError,
    #[error("Ciphersuite Decoding Error: {}", info)]
    CiphersuiteDecodingError { info: String },

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

    #[error("Generic libcrux error")]
    LibcruxError,
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
