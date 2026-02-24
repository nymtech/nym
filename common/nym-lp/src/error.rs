// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::replay::ReplayError;
use libcrux_psq::handshake::HandshakeError;
use libcrux_psq::handshake::builders::BuilderError;
use libcrux_psq::session::SessionError;
use nym_crypto::asymmetric::ed25519::Ed25519RecoveryError;
use nym_kkt::error::KKTError;
use nym_kkt_ciphersuite::{HashFunction, KEM};
use nym_lp_packet::MalformedLpPacketError;
use nym_lp_transport::LpTransportError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LpError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Replay detected: {0}")]
    Replay(#[from] ReplayError),

    #[error("Insufficient buffer size provided")]
    InsufficientBufferSize,

    #[error("Attempted operation on closed session")]
    SessionClosed,

    #[error("There already exists an LP session with receiver index {0}")]
    DuplicateSessionId(u64),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid state transition: tried input {input:?} in state {state:?}")]
    InvalidStateTransition { state: String, input: String },

    #[error("Invalid payload size: expected {expected}, got {actual}")]
    InvalidPayloadSize { expected: usize, actual: usize },

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error(transparent)]
    InvalidBase58String(#[from] bs58::decode::Error),

    /// Session ID from incoming packet does not match any known session.
    #[error("Received packet with unknown session ID: {0}")]
    UnknownSessionId(u64),

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
    StateMachineNotFound { lp_id: u64 },

    /// Ed25519 to X25519 conversion error.
    #[error("Ed25519 key conversion error: {0}")]
    Ed25519RecoveryError(#[from] Ed25519RecoveryError),

    #[error("attempted to create an LP responder without providing a valid KEM keys")]
    ResponderWithMissingKEMKeys,

    #[error(
        "there are no known digests for remote's KEM key with {kem} KEM and {hash_function} hash function"
    )]
    NoKnownKEMKeyDigests {
        kem: KEM,
        hash_function: HashFunction,
    },

    #[error("failed to complete KKT/PSQ handshake: {0}")]
    KKTPSQHandshake(String),

    #[error("failed to complete the KKT exchange: {source}")]
    KKTFailure {
        #[from]
        source: KKTError,
    },

    #[error(transparent)]
    MalformedPacket(#[from] MalformedLpPacketError),

    #[error("version {version} is not supported")]
    UnsupportedVersion { version: u8 },

    #[error("failed to build PSQ responder: {inner:?}")]
    PSQResponderBuilderFailure { inner: BuilderError },

    #[error("failed to build PSQ initiator: {inner:?}")]
    PSQInitiatorBuilderFailure { inner: BuilderError },

    #[error("failed to complete the PSQ handshake: {inner:?}")]
    PSQHandshakeFailure { inner: HandshakeError },

    #[error("failed to run the PSQ session: {inner:?}")]
    PSQSessionFailure { inner: SessionError },

    #[error("failed to derive a transport channel: {inner:?}")]
    TransportDerivationFailure { inner: SessionError },

    #[error("the initiator authenticator is not available after ingesting PSQ msg1")]
    MissingInitiatorAuthenticator,

    #[error("transport failure: {0}")]
    TransportFailure(#[from] LpTransportError),

    #[error("the current session is not in transport state")]
    NotInTransport,
}

impl LpError {
    pub fn kkt_psq_handshake(msg: impl Into<String>) -> Self {
        Self::KKTPSQHandshake(msg.into())
    }
}

impl From<HandshakeError> for LpError {
    fn from(handshake_error: HandshakeError) -> Self {
        Self::PSQHandshakeFailure {
            inner: handshake_error,
        }
    }
}

impl From<SessionError> for LpError {
    fn from(session_error: SessionError) -> Self {
        Self::PSQSessionFailure {
            inner: session_error,
        }
    }
}
