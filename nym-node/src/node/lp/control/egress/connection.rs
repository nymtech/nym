// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::control::LpConnectionStats;
use crate::node::lp::directory::LpNodeDetails;
use crate::node::lp::error::LpHandlerError;
use crate::node::lp::forwarding::client_connection::NestedClientConnectionSender;
use crate::node::lp::state::SharedLpNodeControlState;
use nym_lp::LpTransportSession;
use nym_lp::peer_config::LpReceiverIndex;
use nym_lp::transport::LpHandshakeChannel;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::{debug, warn};

pub(crate) type NestedNodeConnectionSender = ();
pub(crate) type NestedNodeConnectionReceiver = ();

pub(crate) type NestedNodeControlSender = ();
pub(crate) type NestedNodeControlReceiver = ();

/// Initial connection handler for an egress LP node before completing the KKT/PSQ handshake.

pub struct InitialLpEgressNodeConnectionHandler<S> {
    stream: S,
    remote_addr: SocketAddr,
    responder_details: LpNodeDetails,

    state: SharedLpNodeControlState,
    stats: LpConnectionStats,
}

impl<S> InitialLpEgressNodeConnectionHandler<S>
where
    S: LpHandshakeChannel + LpHandshakeChannel + Unpin,
{
    pub(crate) fn new(
        stream: S,
        remote_addr: SocketAddr,
        responder_details: LpNodeDetails,
        state: SharedLpNodeControlState,
    ) -> Self {
        Self {
            stream,
            remote_addr,
            responder_details,
            state,
            stats: LpConnectionStats::new(),
        }
    }

    pub(crate) async fn complete_initial_handshake(
        mut self,
    ) -> Option<Result<LpTransportSession, LpHandlerError>> {
        let remote = self.remote_addr;

        if self.responder_details.kem_key_hashes.is_empty() {
            return Some(Err(LpHandlerError::MissingNodeKEMKeyHashes {
                node_ip: self.remote_addr.ip(),
                node_id: self.responder_details.node_id,
            }));
        }

        // 1. complete KKT/PSQ handshake before doing anything else.
        // bail if it takes too long
        let timeout = self.state.shared.lp_config.debug.handshake_ttl;
        let stream = &mut self.stream;

        let handshake_state = match LpTransportSession::psq_handshake_initiator_mutual(
            stream,
            self.state.local_lp_peer.clone(),
            self.responder_details.to_lp_peer(),
            self.responder_details.supported_protocol,
        ) {
            Ok(handshake_state) => handshake_state,
            Err(err) => {
                debug!("failed to initiate mutual KTT/PSQ handshake with {remote}: {err}");
                self.stats.emit_lifecycle_node_metrics(false);
                return None;
            }
        };

        let session = match tokio::time::timeout(timeout, handshake_state.complete_handshake())
            .await
        {
            Err(_timeout) => {
                debug!("timed out attempting to complete mutual KTT/PSQ handshake with {remote}");
                self.stats.emit_lifecycle_node_metrics(false);
                return None;
            }
            Ok(Err(handshake_failure)) => {
                debug!(
                    "failed to complete mutual KKT/PSQ handshake with {remote}: {handshake_failure}"
                );
                self.stats.emit_lifecycle_node_metrics(false);
                return None;
            }
            Ok(Ok(session)) => session,
        };

        debug!(
            "completed egress KKT/PSQ handshake with node {}: {remote}",
            self.responder_details.node_id
        );

        // TODO: return session, etc.
        Some(Ok(session))
    }
}

pub(crate) struct NestedNodeConnectionHandler<S> {
    /// Persistent connection to exit gateway for forwarding.
    /// Currently, it uses raw TCP socket, later it will be wrapped with dedicated PSQ tunnel
    exit_stream: S,

    /// Socket address of the remote of the established stream
    exit_address: SocketAddr,

    /// Map of senders to each known client handle (based on the inner receiver index)
    client_handles: HashMap<LpReceiverIndex, NestedClientConnectionSender>,

    /// Channel for receiving requests that are to be forwarded into the exit stream
    data_receiver: NestedNodeConnectionReceiver,

    /// Channel for adding new client handle and handling control requests from `NestedConnectionsController`
    control_receiver: NestedNodeControlReceiver,
}

impl<S> NestedNodeConnectionHandler<S>
where
// S: LpTransport + Unpin,
{
    /// Attempt to extract outer receiver index from the received message
    /// (that is meant to be an `LpPacket`)
    fn extract_receiver_index(&self, raw: &[u8]) -> Option<LpReceiverIndex> {
        if raw.len() < 4 {
            return None;
        }
        Some(LpReceiverIndex::from_le_bytes([
            raw[0], raw[1], raw[2], raw[3],
        ]))
    }

    /// Attempt to forward received packet to the client that established the inner LP session
    async fn handle_exit_packet(&self, packet: Vec<u8>) {
        let Some(receiver_index) = self.extract_receiver_index(&packet) else {
            warn!("{} has sent us an invalid LP packet", self.exit_address);
            return;
        };
        let Some(client_handle) = self.client_handles.get(&receiver_index) else {
            warn!(
                "no client handle for receiver index {receiver_index} received from {}",
                self.exit_address
            );
            return;
        };
        // client_handle.send(packet).await;
    }

    async fn run(&mut self) {
        // loop {
        //     tokio::select! {
        //
        //     }
        // }
    }
}
