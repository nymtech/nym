// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{noise_protocol::NoiseError, replay::ReplayError};
use nym_crypto::asymmetric::ed25519::Ed25519RecoveryError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LpError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Snow Error: {0}")]
    SnowKeyError(#[from] snow::Error),

    #[error("Snow Pattern Error: {0}")]
    SnowPatternError(String),

    #[error("Noise Protocol Error: {0}")]
    NoiseError(#[from] NoiseError),

    #[error("Replay detected: {0}")]
    Replay(#[from] ReplayError),

    #[error("Invalid packet format: {0}")]
    InvalidPacketFormat(String),

    #[error("Invalid message type: {0}")]
    InvalidMessageType(u32),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(usize),

    #[error("Insufficient buffer size provided")]
    InsufficientBufferSize,

    #[error("Attempted operation on closed session")]
    SessionClosed,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid state transition: tried input {input:?} in state {state:?}")]
    InvalidStateTransition { state: String, input: String },

    #[error("Invalid payload size: expected {expected}, got {actual}")]
    InvalidPayloadSize { expected: usize, actual: usize },

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("KKT protocol error: {0}")]
    KKTError(String),

    #[error(transparent)]
    InvalidBase58String(#[from] bs58::decode::Error),

    /// Session ID from incoming packet does not match any known session.
    #[error("Received packet with unknown session ID: {0}")]
    UnknownSessionId(u32),

    /// Invalid state transition attempt in the state machine.
    #[error("Invalid input '{input}' for current state '{state}'")]
    InvalidStateTransitionAttempt { state: String, input: String },

    /// Session is closed.
    #[error("Session is closed")]
    LpSessionClosed,

    /// Session is processing an input event.
    #[error("Session is processing an input event")]
    LpSessionProcessing,

    /// State machine not found.
    #[error("State machine not found for lp_id: {lp_id}")]
    StateMachineNotFound { lp_id: u32 },

    /// Ed25519 to X25519 conversion error.
    #[error("Ed25519 key conversion error: {0}")]
    Ed25519RecoveryError(#[from] Ed25519RecoveryError),

    /// Outer AEAD authentication tag verification failed.
    #[error("AEAD authentication tag verification failed")]
    AeadTagMismatch,
}
