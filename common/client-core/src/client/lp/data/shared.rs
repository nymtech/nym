// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use nym_lp::LpTransportSession;
use nym_lp_data::packet::header::LpReceiverIndex;
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};
use nym_lp_gateway_client::{extract_forwarded_response, prepare_send_packet};
use nym_sphinx::addressing::nodes::NodeIdentity;

use crate::client::lp::data::handler::error::LpDataHandlerError;

/// An established LP session with one gateway, and where its data packets go.
///
/// Both halves are here rather than with the transport, which holds neither: it moves packets
/// someone else has encrypted, to an address someone else names.
pub struct LpGatewaySession {
    /// Which gateway this is with, and what a dial for it would ask topology about.
    pub gateway: NodeIdentity,

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
/// Keyed by receiver index, the only thing an arriving packet knows about its session. A gateway is
/// *chosen* by identity and a packet on its way out is *addressed*; both of those find their session
/// by scanning, which is cheap because a client holds a handful of sessions, one per gateway.
///
/// Deliberately not `nym-node`'s `ActiveLpSessions`: TTLs, demotion and multi-peer indexing are
/// answers to a node's problem of holding sessions it did not ask for. A client holds a handful it
/// established itself.
#[derive(Clone, Default)]
pub struct LpGatewaySessions {
    by_index: Arc<DashMap<LpReceiverIndex, LpGatewaySession>>,
}

impl LpGatewaySessions {
    /// Hold this session, in place of any earlier one with the same gateway.
    ///
    /// One per gateway, so that finding a gateway's session by identity or address is never a
    /// choice between two.
    pub fn insert(&self, session: LpGatewaySession) {
        self.by_index
            .retain(|_, existing| existing.gateway != session.gateway);
        self.by_index
            .insert(session.session.receiver_index(), session);
    }

    /// Where this gateway's data packets go, if we hold a session with it at all.
    ///
    /// The one crossing between naming a gateway and addressing it: a caller that has picked a
    /// gateway resolves it here, and what comes back is what the outbound pipeline carries from
    /// there on.
    pub(crate) fn data_address(&self, gateway: NodeIdentity) -> Option<SocketAddr> {
        self.by_index
            .iter()
            .find(|entry| entry.gateway == gateway)
            .map(|entry| entry.data_address)
    }

    /// Encrypt a frame on the session with whichever gateway answers to this address.
    pub(crate) fn prepare(
        &self,
        gateway: SocketAddr,
        frame: LpFrame,
    ) -> Result<EncryptedLpPacket, LpDataHandlerError> {
        let mut session = self
            .by_index
            .iter_mut()
            .find(|entry| entry.data_address == gateway)
            .ok_or_else(|| {
                LpDataHandlerError::other(format!("no LP session with a gateway at {gateway}"))
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
#[derive(Clone)]
pub struct SharedLpDataState {
    inner: Arc<SharedLpDataStateInner>,
}
struct SharedLpDataStateInner {
    /// The sessions this client holds with its gateways.
    sessions: LpGatewaySessions,
}

impl SharedLpDataState {
    pub fn new(sessions: LpGatewaySessions) -> Self {
        SharedLpDataState {
            inner: Arc::new(SharedLpDataStateInner { sessions }),
        }
    }

    pub(crate) fn sessions(&self) -> &LpGatewaySessions {
        &self.inner.sessions
    }
}
