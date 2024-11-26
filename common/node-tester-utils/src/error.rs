// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NodeId;
use nym_sphinx::chunking::ChunkingError;
use nym_sphinx::receiver::MessageRecoveryError;
use nym_topology::NymTopologyError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkTestingError {
    #[error(transparent)]
    SerializationFailure(#[from] serde_json::Error),

    #[error("could not recover received test message: {source}")]
    MalformedTestMessageReceived { source: serde_json::Error },

    #[error(transparent)]
    InvalidTopology(#[from] NymTopologyError),

    #[error("The specified mixnode (id: {mix_id}) doesn't exist")]
    NonExistentMixnode { mix_id: NodeId },

    #[error("The specified mixnode (identity: {mix_identity}) doesn't exist")]
    NonExistentMixnodeIdentity { mix_identity: String },

    #[error("The specified gateway (id: {gateway_identity}) doesn't exist")]
    NonExistentGateway { gateway_identity: String },

    #[error("The provided test message is too long to fit in a single sphinx packet")]
    TestMessageTooLong,

    #[error(
        "could not recover underlying data from the received packet since it was malformed: {source}"
    )]
    MalformedPacketReceived {
        #[from]
        source: MessageRecoveryError,
    },

    #[error("Received ack packet could not be recovered")]
    UnrecoverableAck,

    #[error("could not recover ack FragmentIdentifier: {source}")]
    MalformedAckIdentifier { source: ChunkingError },

    #[error("received a packet that could not be reconstructed into a full message with a single fragment")]
    NonReconstructablePacket,

    #[error("the recipient of the test packet was never specified")]
    UnknownPacketRecipient,
}
