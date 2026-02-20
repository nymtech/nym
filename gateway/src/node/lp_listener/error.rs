// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp::state_machine::{LpAction, LpDataKind};
use nym_lp::{LpError, MalformedLpPacketError};
use nym_lp_transport::LpTransportError;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LpHandlerError {
    #[error("failed to establish egress connection to {egress}: {reason}")]
    ConnectionFailure { egress: SocketAddr, reason: String },

    #[error(transparent)]
    LpTransportError(#[from] LpTransportError),

    #[error("missing session state for {receiver_index} - has it been removed due to inactivity?")]
    MissingLpSession { receiver_index: u32 },

    #[error(transparent)]
    LpProtocolError(#[from] LpError),

    #[error("no action has been emitted from the LP State Machine")]
    UnexpectedStateMachineHalt,

    #[error("the state machine instructed an unexpected action: {action:?}")]
    UnexpectedStateMachineAction { action: LpAction },

    #[error("received registration request was malformed: {source}")]
    MalformedRegistrationRequest { source: bincode::Error },

    #[error("received a malformed packet: {0}")]
    MalformedLpPacket(#[from] MalformedLpPacketError),

    #[error("received payload type of an unexpected type: {typ:?}")]
    UnexpectedLpPayload { typ: LpDataKind },

    #[error("timed out while attempting to send to/receive from the connection")]
    ConnectionTimeout,

    #[error("{0}")]
    Other(String),
}

impl LpHandlerError {
    pub fn is_connection_closed(&self) -> bool {
        match self {
            LpHandlerError::LpTransportError(transport_err) => match transport_err {
                LpTransportError::ConnectionClosed => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn other(msg: impl Into<String>) -> Self {
        LpHandlerError::Other(msg.into())
    }
}
