use std::array::TryFromSliceError;

use crate::constants::MIN_MESSAGE_LEN;
use crate::constants::MIX_PARAMS_LEN;
use chacha20::cipher::InvalidLength;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum OutfoxError {
    #[error("Lengths mismatch, expected: {expected}, got: {got}")]
    LenMismatch { expected: usize, got: usize },
    #[error("{source}")]
    ChaCha20InvalidLength {
        #[from]
        source: InvalidLength,
    },
    #[error("ChaCha20Poly1305 - {0}")]
    ChaCha20Poly1305Error(String),
    #[error("Key length must be 32 bytes")]
    InvalidKeyLength,
    #[error("Message length must be greater then {MIN_MESSAGE_LEN} bytes")]
    InvalidMessageLength,
    #[error("{source}")]
    TryFromSlice {
        #[from]
        source: TryFromSliceError,
    },
    #[error("Header length must be {MIX_PARAMS_LEN}, got {0}")]
    InvalidHeaderLength(usize),
}
