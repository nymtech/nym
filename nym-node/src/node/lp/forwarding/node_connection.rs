// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::lp::forwarding::client_connection::NestedClientConnectionSender;
use nym_lp::peer_config::LpReceiverIndex;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::warn;

pub(crate) type NestedNodeConnectionSender = ();
pub(crate) type NestedNodeConnectionReceiver = ();

pub(crate) type NestedNodeControlSender = ();
pub(crate) type NestedNodeControlReceiver = ();

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
