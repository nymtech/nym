// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use nym_lp::LpTransportSession;
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_lp_gateway_client::{extract_forwarded_response, prepare_send_packet};

use crate::client::lp::data::handler::error::LpDataHandlerError;

/// An established LP session with one gateway, and where its data packets go.
///
/// Both halves are here rather than with the transport, which holds neither: it moves packets
/// someone else has encrypted, to an address someone else names.
pub struct LpGatewaySession {
    /// Named by the gateway during registration, so it can be addressed as well as read.
    pub session: LpTransportSession,

    /// The gateway's UDP address, not the TCP one the handshake ran on.
    pub data_address: SocketAddr,
}

/// The sessions a client holds with gateways.
///
/// Shared rather than owned because both directions need to mutate a session: the handler encrypts
/// outbound frames on it, and every inbound worker decrypts on it. `DashMap` gives that per-entry,
/// which is what lets the whole unwrapping pipeline live in a worker.
///
/// Two indexes because the two directions ask different questions. An arriving packet knows only
/// the receiver index in its outer header; an outbound packet knows only the address it is going
/// to, which is what the pipeline threads through as its destination.
///
/// Deliberately not `nym-node`'s `ActiveLpSessions`: TTLs, demotion and multi-peer indexing are
/// answers to a node's problem of holding sessions it did not ask for. A client holds a handful it
/// established itself.
#[derive(Clone, Default)]
pub struct LpGatewaySessions {
    by_index: Arc<DashMap<LpReceiverIndex, LpGatewaySession>>,
    by_address: Arc<DashMap<SocketAddr, LpReceiverIndex>>,
}

impl LpGatewaySessions {
    pub fn insert(&self, session: LpGatewaySession) {
        let index = session.session.receiver_index();
        self.by_address.insert(session.data_address, index);
        self.by_index.insert(index, session);
    }

    /// Any gateway we hold a session with.
    ///
    /// For the [`InputMessage`] adapter alone, which has no field to name one with - everything
    /// else says where its message goes, and this store only resolves what it is told. "Any" is
    /// the honest word: that path has nothing to choose on, and it goes when the adapter does.
    ///
    /// [`InputMessage`]: crate::client::inbound_messages::InputMessage
    pub(crate) fn any_gateway(&self) -> Option<SocketAddr> {
        self.by_address.iter().next().map(|entry| *entry.key())
    }

    /// Encrypt a frame on the session with whichever gateway answers to this address.
    pub(crate) fn prepare(
        &self,
        gateway: SocketAddr,
        frame: LpFrame,
    ) -> Result<EncryptedLpPacket, LpDataHandlerError> {
        let index = *self.by_address.get(&gateway).ok_or_else(|| {
            LpDataHandlerError::other(format!("no LP session with a gateway at {gateway}"))
        })?;

        let mut session = self.by_index.get_mut(&index).ok_or_else(|| {
            LpDataHandlerError::internal(format!("session {index} is indexed but missing"))
        })?;

        prepare_send_packet(frame, &mut session.session)
            .map_err(|source| LpDataHandlerError::other(format!("could not encrypt: {source}")))
    }

    /// Decrypt an arriving packet on whichever of our sessions it names.
    pub(crate) fn receive(&self, packet: EncryptedLpPacket) -> Result<LpFrame, LpDataHandlerError> {
        let index = packet.outer_header().receiver_idx;

        let mut session = self.by_index.get_mut(&index).ok_or_else(|| {
            LpDataHandlerError::other(format!(
                "no session of ours answers to receiver index {index}"
            ))
        })?;

        extract_forwarded_response(packet, &mut session.session)
            .map_err(|source| LpDataHandlerError::other(format!("could not decrypt: {source}")))
    }
}

/// What both directions of the data plane share.
///
/// Held behind an `Arc` and handed to each direction whole.
/// Anything only one direction touches belongs to that direction instead.
///
/// The state itself is never mutated: what has to change - a session's counters, a reassembly
/// buffer - carries its own interior sharing, which is what lets a whole pipeline run in a worker.
pub struct SharedLpDataState {
    /// The sessions this client holds with its gateways.
    pub(crate) sessions: LpGatewaySessions,
}

impl SharedLpDataState {
    pub fn new(sessions: LpGatewaySessions) -> Self {
        SharedLpDataState { sessions }
    }
}
