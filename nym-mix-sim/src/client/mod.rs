// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    time::Instant,
};

use nym_lp_data::AddressedTimedData;

use crate::{node::NodeId, packet::WirePacketFormat};

pub mod nymnode;
pub mod simple;
pub mod sphinx;

/// Compact identifier for a simulated client.
pub type ClientId = NodeId;

/// Driver-facing interface for a simulated client.
///
/// Erases `Fr`, `Pkt`, and `Mk` so that [`MixSimDriver`] only needs `Ts`.
/// Implemented by [`simple::SimpleClient`] and any other concrete client types.
///
/// [`MixSimDriver`]: crate::driver::MixSimDriver
pub trait MixSimClient: Send {
    fn tick(&mut self, timestamp: Instant);
}

/// Pipeline interface used by [`BaseClient`] to convert raw app payloads into
/// wire packets and to unwrap received packets back into plaintext.
///
/// `SndPkt` is the outgoing packet type (e.g. [`SimplePacket`] or
/// [`SimMixPacket`]).  `RcvPkt` defaults to `SndPkt` but can differ when the
/// inbound and outbound wire formats diverge (e.g. the Sphinx client receives
/// raw `Vec<u8>` final-hop payloads from nodes).
///
/// [`SimplePacket`]: crate::packet::simple::SimplePacket
/// [`SimMixPacket`]: crate::packet::sphinx::SimMixPacket
pub trait ProcessingClient<SndPkt, RcvPkt = SndPkt>: Send {
    /// Wrap `input` into one or more outbound packets addressed toward `dst`.
    fn process(
        &mut self,
        input: Vec<u8>,
        dst: ClientId,
        timestamp: Instant,
    ) -> Vec<AddressedTimedData<SndPkt>>;

    /// Unwrap an inbound packet received from the mix network.
    ///
    /// Returns `Ok(Some(plaintext))` for a real message, `Ok(None)` when the
    /// packet is cover traffic or an incomplete fragment, and `Err` when
    /// decryption or deserialisation fails.
    fn unwrap(&mut self, input: RcvPkt, timestamp: Instant) -> anyhow::Result<Option<Vec<u8>>>;
}

/// Shared UDP transport layer for simulated clients.
///
/// Encapsulates both sockets, the routing directory, and the client id so that
/// multiple concrete client types can reuse `send_to_node`, `recv_from_mix`,
/// and `recv_from_app` without duplicating that logic.  Packet types are
/// method-level generics so `BaseClient` itself has no type parameters.
pub struct BaseClient<Pc, SndPkt, RcvPkt = SndPkt> {
    /// Identifier of this client within the topology.
    id: ClientId,
    /// Socket bound to the mix-network address; sends to first-hop nodes and
    /// receives final-hop packets.
    mix_socket: UdpSocket,
    /// Socket bound to the app address; receives application payloads from
    /// external CLIs (e.g. `mix-client`).
    app_socket: UdpSocket,

    /// Packets that have been processed and are waiting to be forwarded to their
    /// first-hop node, sorted (loosely) by scheduled send timestamp.
    outgoing_queue: Vec<AddressedTimedData<SndPkt>>,

    /// Concrete client-processing implementation invoked from each tick phase.
    processing_client: Pc,

    /// Phantom data to carry the `RcvPkt` type parameter without storing a value.
    _marker: std::marker::PhantomData<RcvPkt>,
}

impl<Pc, SndPkt, RcvPkt> BaseClient<Pc, SndPkt, RcvPkt> {
    /// Bind both UDP sockets to the given addresses.
    pub(crate) fn with_pipeline(
        client_id: ClientId,
        mixnet_address: SocketAddr,
        app_address: SocketAddr,
        processing_client: Pc,
    ) -> anyhow::Result<Self> {
        let mix_socket = UdpSocket::bind(mixnet_address)?;
        mix_socket.set_nonblocking(true)?;

        let app_socket = UdpSocket::bind(app_address)?;
        app_socket.set_nonblocking(true)?;

        Ok(Self {
            id: client_id,
            mix_socket,
            app_socket,
            outgoing_queue: Vec::new(),
            processing_client,
            _marker: std::marker::PhantomData,
        })
    }
}

impl<Pc, SndPkt, RcvPkt> BaseClient<Pc, SndPkt, RcvPkt>
where
    SndPkt: WirePacketFormat,
    RcvPkt: WirePacketFormat,
{
    /// Send `packet` to the mix node identified by `node_id` via `mix_socket`.
    ///
    /// Resolves `node_id` against the shared [`crate::topology::directory::Directory`], serialises via
    /// [`WirePacketFormat::to_bytes`], and dispatches with a single `sendto`.
    /// Errors are logged but not propagated.
    pub fn send_to_node(&self, node_address: SocketAddr, packet: SndPkt) {
        if let Err(e) = self.mix_socket.send_to(&packet.to_bytes(), node_address) {
            tracing::error!(
                "[Client {}] Failed to send to node @ {node_address}: {e}",
                self.id
            );
        } else {
            tracing::debug!("[Client {}] Sent packet to node @ {node_address}", self.id);
        }
    }

    /// Attempt to receive one packet from the mix socket and deserialise it.
    ///
    /// Returns `None` when the socket would block (no datagram waiting).
    pub fn recv_from_mix(&self) -> Option<anyhow::Result<RcvPkt>> {
        let mut buf = [0u8; 1500];
        let (nb, src) = match self.mix_socket.recv_from(&mut buf) {
            Ok(r) => r,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return None,
            Err(e) => {
                tracing::error!("[Client {}] mix_socket recv error: {e}", self.id);
                return None;
            }
        };
        tracing::debug!(
            "[Client {}] Received {nb} byte(s) from mix node {src}",
            self.id
        );
        Some(RcvPkt::try_from_bytes(&buf[..nb]))
    }

    /// Attempt to receive one raw datagram from the app socket.
    ///
    /// Returns `None` when the socket would block (no datagram waiting).
    pub fn recv_from_app(&self) -> Option<anyhow::Result<Vec<u8>>> {
        let mut buf = [0u8; 15000];
        let nb = match self.app_socket.recv(&mut buf) {
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return None,
            Err(e) => {
                tracing::error!("[Client {}] app_socket recv error: {e}", self.id);
                return None;
            }
        };
        Some(Ok(buf[..nb].to_vec()))
    }
}

impl<Pc, SndPkt, RcvPkt> MixSimClient for BaseClient<Pc, SndPkt, RcvPkt>
where
    SndPkt: WirePacketFormat + Debug + Send,
    RcvPkt: WirePacketFormat + Debug + Send,
    Pc: ProcessingClient<SndPkt, RcvPkt>,
{
    fn tick(&mut self, timestamp: Instant) {
        self.tick_app_incoming(timestamp);
        self.tick_outgoing(timestamp);
        self.tick_mix_incoming(timestamp);
    }
}

impl<Pc, SndPkt, RcvPkt> BaseClient<Pc, SndPkt, RcvPkt>
where
    SndPkt: WirePacketFormat + Debug + Send,
    RcvPkt: WirePacketFormat + Debug + Send,
    Pc: ProcessingClient<SndPkt, RcvPkt>,
{
    /// **Phase 1 — app incoming**: drain the app socket, run each payload
    /// through the processing pipeline, and enqueue the resulting packets.
    fn tick_app_incoming(&mut self, timestamp: Instant) {
        // Collect (dst, payload) pairs from the app socket.
        let mut inputs = Vec::new();
        while let Some(result) = self.recv_from_app() {
            let bytes = match result {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("[Client {}] app_socket recv error: {e}", self.id);
                    continue;
                }
            };

            // We assume format is [dst, payload]
            if bytes.len() < 2 {
                tracing::warn!(
                    "[Client {}] app message too short ({} bytes), dropping",
                    self.id,
                    bytes.len()
                );
                continue;
            }

            let dst = bytes[0];
            let payload = bytes[1..].to_vec();
            tracing::debug!(
                "[Client {}] App input: {} byte(s) → client {dst}",
                self.id,
                payload.len()
            );
            inputs.push((dst, payload));
        }

        // Always call process at least once; use an empty payload to self when idle.
        // We need to tick cover traffic
        if inputs.is_empty() {
            inputs.push((self.id, vec![]));
        }

        for (dst, payload) in inputs {
            let packets = self.processing_client.process(payload, dst, timestamp);
            self.outgoing_queue.extend(packets);
        }
    }

    /// **Phase 2 — outgoing**: send all queued packets whose scheduled
    /// timestamp is ≤ `timestamp` to their first-hop node.
    fn tick_outgoing(&mut self, timestamp: Instant) {
        let to_send = self
            .outgoing_queue
            .extract_if(.., |pkt| pkt.data.timestamp <= timestamp)
            .collect::<Vec<_>>();
        for pkt in to_send {
            self.send_to_node(pkt.dst, pkt.data.data);
        }
    }

    /// **Phase 3 — mix incoming**: drain the mix socket and pass each packet
    /// through the unwrapping pipeline.
    fn tick_mix_incoming(&mut self, timestamp: Instant) {
        while let Some(result) = self.recv_from_mix() {
            match result {
                Ok(pkt) => match self.processing_client.unwrap(pkt, timestamp) {
                    Ok(Some(content)) => {
                        tracing::info!(
                            "[Client {}] Received: {:?}",
                            self.id,
                            String::from_utf8_lossy(&content)
                        );
                    }
                    Err(e) => {
                        tracing::error!("[Client {}] Error unwrapping packet : {e}", self.id);
                    }
                    Ok(None) => {}
                },
                Err(e) => {
                    tracing::error!("[Client {}] Failed to deserialize mix packet: {e}", self.id);
                }
            }
        }
    }
}
