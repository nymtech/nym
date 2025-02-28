// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::SharedKeyUsageError;
use nym_credentials_interface::CompactEcashError;
use nym_crypto::asymmetric::ed25519::SignatureError;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddressError;
use nym_sphinx::forwarding::packet::MixPacketFormattingError;
use nym_sphinx::params::packet_sizes::PacketSize;
use serde::{Deserialize, Serialize};
use std::string::FromUtf8Error;
use thiserror::Error;

// specific errors (that should not be nested!!) for clients to match on
#[derive(Debug, Copy, Clone, Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimpleGatewayRequestsError {
    #[error("insufficient bandwidth available to process the request. required: {required}B, available: {available}B")]
    OutOfBandwidth { required: i64, available: i64 },

    #[error("the provided ticket has already been spent before at this gateway")]
    TicketReplay,
}

impl SimpleGatewayRequestsError {
    pub fn is_ticket_replay(&self) -> bool {
        matches!(self, SimpleGatewayRequestsError::TicketReplay)
    }
}

#[derive(Debug, Error)]
pub enum GatewayRequestsError {
    #[error(transparent)]
    KeyUsageFailure(#[from] SharedKeyUsageError),

    #[error("the received request is malformed: {source}")]
    MalformedRequest { source: serde_json::Error },

    #[error("the received response is malformed: {source}")]
    MalformedResponse { source: serde_json::Error },

    #[error("received request with an unknown kind: {kind}")]
    UnknownRequestKind { kind: u8 },

    #[error("received response with an unknown kind: {kind}")]
    UnknownResponseKind { kind: u8 },

    #[error("the encryption flag had an unexpected value")]
    InvalidEncryptionFlag,

    #[error("the request is too short")]
    TooShortRequest,

    #[error("provided MAC is invalid")]
    InvalidMac,

    #[error("address field was incorrectly encoded: {source}")]
    IncorrectlyEncodedAddress {
        #[from]
        source: NymNodeRoutingAddressError,
    },

    #[error("received request had invalid size. (actual: {0}, but expected one of: {a} (ACK), {r} (REGULAR), {e8}, {e16}, {e32} (EXTENDED))",
        a = PacketSize::AckPacket.size(),
        r = PacketSize::RegularPacket.size(),
        e8 = PacketSize::ExtendedPacket8.size(),
        e16 = PacketSize::ExtendedPacket16.size(),
        e32 = PacketSize::ExtendedPacket32.size())
    ]
    RequestOfInvalidSize(usize),

    #[error("received sphinx packet was malformed")]
    MalformedSphinxPacket,

    #[error("failed to serialise created sphinx packet: {0}")]
    SphinxSerialisationFailure(#[from] MixPacketFormattingError),

    #[error("the received encrypted data was malformed")]
    MalformedEncryption,

    #[error("provided packet mode is invalid")]
    InvalidPacketMode,

    #[error("failed to deserialize provided credential: {0}")]
    EcashCredentialDeserializationFailure(#[from] CompactEcashError),

    #[error("failed to deserialize provided credential: EOF")]
    CredentialDeserializationFailureEOF,

    #[error("failed to deserialize provided credential: malformed string: {0}")]
    CredentialDeserializationFailureMalformedString(#[from] FromUtf8Error),

    #[error("the provided [v1] credential has invalid number of parameters - {0}")]
    InvalidNumberOfEmbededParameters(u32),

    #[error("failed to authenticate the client: {0}")]
    Authentication(#[from] AuthenticationFailure),

    // variant to catch legacy errors
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum AuthenticationFailure {
    #[error(transparent)]
    KeyUsageFailure(#[from] SharedKeyUsageError),

    #[error("failed to verify provided address ciphertext")]
    MalformedCiphertext,

    #[error("failed to verify request signature")]
    InvalidSignature(#[from] SignatureError),

    #[error("provided request timestamp is in the future")]
    RequestTimestampInFuture,

    #[error("the client is not registered")]
    NotRegistered,

    #[error("the provided request is too stale to process")]
    StaleRequest,
}
