use crate::lion::MIN_MESSAGE_LEN;
use chacha20::cipher::InvalidLength;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutfoxError {
    #[error("Lengths mismatch, expected: {expected}, got: {got}")]
    LenMismatch { expected: usize, got: usize },
    #[error("{source}")]
    ChaCha20InvalidLength {
        #[from]
        source: InvalidLength,
    },
    #[error("ChaCha20Poly1305 - Opaque error")]
    ChaCha20Poly1305Error,
    #[error("Key length must be 32 bytes")]
    InvalidKeyLength,
    #[error("Message length must be greater then {MIN_MESSAGE_LEN} bytes")]
    InvalidMessageLength,
}
