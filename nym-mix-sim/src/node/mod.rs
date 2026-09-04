// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::Debug,
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    time::Instant,
};

use nym_lp_data::{
    AddressedTimedData,
    common::traits::{Transport, TransportUnwrap},
    nymnodes::traits::NymNodeProcessingPipeline,
};

use crate::packet::WirePacketFormat;

pub mod nymnode;
pub mod simple;
pub mod sphinx;

/// Compact identifier for a mix node in the simulation topology.
///
/// `u8` keeps the IDs small (max 255 nodes) and is large enough for any
/// realistic simulated topology.
pub type NodeId = u8;

/// Driver-facing interface for a mix node.
///
/// Erases `Pkt` and `Pn` so that [`MixSimDriver`] only needs `Ts`.
/// Implemented by [`BaseNode<Ts, Pkt, Pn>`] for any compatible `Pkt` and
/// `Pn`.
///
/// [`MixSimDriver`]: crate::driver::MixSimDriver
pub trait MixSimNode: Send {
    /// **Phase 1** — drain the UDP socket into the inbound buffer
    fn tick_incoming(&mut self);

    /// **Phase 2** — pass every buffered packet through the mix pipeline and
    /// move the results into the outbound queue.
    fn tick_processing(&mut self, timestamp: Instant);

    /// **Phase 3** — forward all outbound packets whose scheduled timestamp is
    /// ≤ `timestamp` to their next-hop address.
    fn tick_outgoing(&mut self, timestamp: Instant);

    /// Pretty-print the node's current buffer state to stdout (used in manual mode).
    fn display_state(&self);
}

/// Full mix-node state: UDP transport, routing directory, packet buffers, and
/// processing pipeline.
///
/// `Pkt` is the wire packet type (e.g. [`SimplePacket`] or [`SimMixPacket`]).
/// `Pn` is any type that implements [`NymNodeProcessingPipeline<Pkt>`].
///
/// Concrete node variants (`SimpleNode`, `SphinxNode`, …) are type aliases
/// over this struct and only need to supply a `new()` constructor that wires
/// up the right pipeline.
///
/// [`SimplePacket`]: crate::packet::simple::SimplePacket
/// [`SimMixPacket`]: crate::packet::sphinx::SimMixPacket
pub struct BaseNode<Pkt, Frame, Pn, NdId = SocketAddr> {
    /// Identifier of this node within the topology.
    pub(crate) id: NodeId,
    /// Notional reliability percentage; not yet used by the simulator but kept
    /// so future tests can drive the reliability layer.
    _reliability: u8,
    /// UDP address this node is bound to.
    pub(crate) socket_address: SocketAddr,
    /// Non-blocking UDP socket used for both receive and send.
    socket: UdpSocket,

    /// Inbound buffer: raw packets drained from the socket in `tick_incoming`,
    /// ready to be fed through the mix pipeline in `tick_processing`.
    packets_to_process: Vec<Pkt>,
    /// Outbound buffer: *frames* produced by the mix pipeline, each tagged with the timestamp
    /// at which it should be released by `tick_outgoing`. Held un-wrapped deliberately — the
    /// transport wrap is applied on release, not here.
    processed_frames: Vec<AddressedTimedData<Frame, NdId>>,

    /// Concrete mix-processing implementation invoked by `tick_processing`.
    processing_node: Pn,
}

impl<Pkt, Frame, Pn, NdId> BaseNode<Pkt, Frame, Pn, NdId> {
    /// Bind a non-blocking UDP socket to `socket_address` and initialise the
    /// node with the given `pipeline`.
    pub(crate) fn with_pipeline(
        id: NodeId,
        reliability: u8,
        socket_address: SocketAddr,
        processing_node: Pn,
    ) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(socket_address)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            id,
            _reliability: reliability,
            socket_address,
            socket,
            packets_to_process: Vec::new(),
            processed_frames: Vec::new(),
            processing_node,
        })
    }

    /// Send `packet` to the destination identified by `address`.
    ///
    /// Serialises via [`WirePacketFormat::to_bytes`], and dispatches with a
    /// single `sendto`. Errors are logged but not propagated.
    pub fn send_to(&self, address: SocketAddr, packet: Pkt)
    where
        Pkt: WirePacketFormat,
    {
        if let Err(e) = self.socket.send_to(&packet.to_bytes(), address) {
            tracing::error!("[Node {}] Failed to send data to {address} : {e}", self.id);
        } else {
            tracing::debug!("[Node {}] Successfully sent a packet to {address}", self.id);
        }
    }

    /// Attempt to receive one UDP datagram and deserialise it as `Pkt`.
    ///
    /// Returns `None` when the socket would block (no datagram waiting).
    pub fn recv_packet(&self) -> Option<anyhow::Result<Pkt>>
    where
        Pkt: WirePacketFormat,
    {
        let mut buf = [0; 1500];
        let (nb_bytes, src_address) = match self.socket.recv_from(&mut buf) {
            Ok(result) => result,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return None,
            Err(e) => {
                tracing::error!("Error receiving packet : {e}");
                return None;
            }
        };
        tracing::debug!(
            "[Node {}] Received {nb_bytes} bytes from {src_address}",
            self.id
        );
        Some(Pkt::try_from_bytes(&buf[..nb_bytes]))
    }
}

impl<Pkt, Frame, Pn, NdId> MixSimNode for BaseNode<Pkt, Frame, Pn, NdId>
where
    Pkt: WirePacketFormat + Debug + Send,
    Frame: Send + Debug,
    NdId: Copy + Debug + Send,
    Pn: NymNodeProcessingPipeline<Frame, NdId>
        + Transport<Pkt, NdId, Frame = Frame>
        + TransportUnwrap<Pkt, Frame = Frame>
        + Send,
    <Pn as Transport<Pkt, NdId>>::Error: Debug,
    <Pn as TransportUnwrap<Pkt>>::Error: Debug,
{
    fn tick_incoming(&mut self) {
        while let Some(maybe_packet) = self.recv_packet() {
            match maybe_packet {
                Ok(packet) => self.packets_to_process.push(packet),
                Err(e) => tracing::error!("[Node {}] Failed to deserialize packet : {e}", self.id),
            }
        }
    }

    /// Strip the transport layer, then mix and re-frame.
    ///
    /// The matching transport *wrap* happens in [`Self::tick_outgoing`] once a frame's
    /// scheduled time arrives — see [`NymNodeProcessingPipeline::process`] for why it must not
    /// happen here.
    fn tick_processing(&mut self, timestamp: Instant) {
        while let Some(packet) = self.packets_to_process.pop() {
            let frame = match self.processing_node.packet_to_frame(packet, timestamp) {
                Ok(frame) => frame,
                Err(e) => {
                    tracing::error!("[Node {}] Failed to decode packet : {e:?}", self.id);
                    continue;
                }
            };

            let frames = self.processing_node.process(frame, timestamp);
            self.processed_frames.extend(frames);
        }
    }

    fn tick_outgoing(&mut self, timestamp: Instant) {
        let mut due = self
            .processed_frames
            .extract_if(.., |frame| frame.data.timestamp <= timestamp)
            .collect::<Vec<_>>();

        // release order, so the transport wrap numbers packets the way they go on the wire
        due.sort_by_key(|frame| frame.data.timestamp);

        for frame in due {
            let dst = frame.dst;
            // the wrap resolves the peer's identity to a wire address, so the send target comes
            // back with the packet rather than from the frame
            match self.processing_node.to_transport_packet(frame) {
                Ok(packet) => self.send_to(packet.dst, packet.data.data),
                Err(e) => {
                    tracing::error!(
                        "[Node {}] Failed to wrap a frame for {dst:?} : {e:?}",
                        self.id
                    )
                }
            }
        }
    }

    fn display_state(&self) {
        println!("│  Node {:2} @ {}", self.id, self.socket_address);
        if self.packets_to_process.is_empty() {
            println!("│    to_process buffer: (empty)");
        } else {
            println!(
                "│    to_process buffer: {} packet(s)",
                self.packets_to_process.len()
            );
            for (i, pkt) in self.packets_to_process.iter().enumerate() {
                println!("│      [{i}] {pkt:#?}");
            }
        }

        if self.processed_frames.is_empty() {
            println!("│    processed buffer: (empty)");
        } else {
            println!(
                "│    processed buffer: {} packet(s)",
                self.processed_frames.len()
            );
            for (i, pkt) in self.processed_frames.iter().enumerate() {
                println!("│      [{i}] {pkt:#?}");
            }
        }
    }
}
