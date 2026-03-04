// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp::packet::message::LpMessageType;
use nym_lp::peer_config::LpReceiverIndex;
use nym_lp::session::LpAction;
use nym_lp::transport::LpTransportError;
use nym_lp::{LpError, packet::MalformedLpPacketError};
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LpHandlerError {
    #[error("failed to establish egress connection to {egress}: {reason}")]
    ConnectionFailure { egress: SocketAddr, reason: String },

    #[error(transparent)]
    LpTransportError(#[from] LpTransportError),

    #[error("missing session state for {receiver_index} - has it been removed due to inactivity?")]
    MissingLpSession { receiver_index: LpReceiverIndex },

    #[error(transparent)]
    LpProtocolError(#[from] LpError),

    #[error("the initial KKT/PSQ handshake has not been completed")]
    IncompleteHandshake,

    #[error("receiver_idx mismatch: connection bound to {established}, packet has {received}")]
    MismatchedReceiverIndex {
        established: LpReceiverIndex,
        received: LpReceiverIndex,
    },

    #[error("the state machine instructed an unexpected action: {action:?}")]
    UnexpectedStateMachineAction { action: LpAction },

    #[error("received registration request was malformed: {source}")]
    MalformedRegistrationRequest { source: bincode::Error },

    #[error("received a malformed packet: {0}")]
    MalformedLpPacket(#[from] MalformedLpPacketError),

    #[error("received payload type of an unexpected type: {typ:?}")]
    UnexpectedLpPayload { typ: LpMessageType },

    #[error("timed out while attempting to send to/receive from the connection")]
    ConnectionTimeout,

    #[error("data channel is not yet implemented")]
    UnimplementedDataChannel,

    #[error("{0}")]
    Other(String),
}

impl LpHandlerError {
    pub fn is_connection_closed(&self) -> bool {
        match self {
            LpHandlerError::LpTransportError(transport_err) => {
                matches!(transport_err, LpTransportError::ConnectionClosed)
            }
            _ => false,
        }
    }

    pub fn other(msg: impl Into<String>) -> Self {
        LpHandlerError::Other(msg.into())
    }
}
