// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The gateway client websocket: the other way into the mixnet.
//!
//! Beside [`sphinx`](super::sphinx) rather than under the Noise files next to it, because this is a
//! second WIRE carrying the same packet format: no Noise handshake, no static keys, authentication by
//! the gateway's own registration handshake instead.
//!
//! A gateway probe crosses the two wires in opposite directions, which is what lets one run measure
//! two interfaces without either leg's losses being attributable to the other:
//!
//! ```text
//! client ingest    agent --ws--> GW --forwarded verbatim--> agent's mixnet listener
//! client delivery  agent --noise--> GW --final hop unwrapped--> this session
//! ```

// SCAFFOLD: the bodies land as group 9's tasks are worked through, at which point both allows come off
#![allow(dead_code, unused_variables)]

use crate::mixnet::client_session::inbox::GatewaySessionInbox;
use futures::stream::SplitSink;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_requests::{GatewayProtocolVersion, SharedSymmetricKey};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

pub(crate) mod events;
pub(crate) mod inbox;
pub(crate) mod reader;

/// The session's transport.
///
/// A plain `TcpStream` and not `MaybeTlsStream`, which makes "no TLS, ever" structural rather than a
/// convention: the agent dials an ip and upgrades it itself, so there is no path on which a
/// certificate or a resolver could enter.
type WsConn = WebSocketStream<TcpStream>;

/// Where a client session is established, and what authenticates the gateway at the far end.
pub(crate) struct GatewaySessionTarget {
    /// `ws://<ip>:<port>`, built from the assignment. Never an announced hostname and never a wss
    /// entry: the source ip the gateway observes has to be genuinely ours for its monitor-session
    /// identification to hold, and no operator's proxy may sit in the middle of a measurement.
    pub(crate) address: SocketAddr,

    /// The gateway's ed25519 identity as bonded in the contract. The registration handshake proves
    /// the far end holds the matching private key, which is what makes TLS unnecessary here.
    pub(crate) identity: ed25519::PublicKey,
}

impl GatewaySessionTarget {
    pub(crate) fn new(address: SocketAddr, identity: ed25519::PublicKey) -> Self {
        GatewaySessionTarget { address, identity }
    }
}

/// The timeouts one session is held to.
///
/// Separate from the probe's config so the wire has no opinion about test profiles, and named rather
/// than passed as three durations, which at this arity is otherwise a swap waiting to happen.
pub(crate) struct GatewaySessionConfig {
    /// Bounds the TCP connect and the websocket upgrade.
    pub(crate) connect_timeout: Duration,

    /// Bounds protocol negotiation and the registration handshake.
    pub(crate) registration_timeout: Duration,

    /// How long the inbox waits for a single delivered payload.
    pub(crate) receive_timeout: Duration,
}

/// A live, registered client session with one gateway.
///
/// Holds the write half only. The read half belongs to a spawned
/// [`SessionReader`](reader::SessionReader) filing onto the [`GatewaySessionInbox`] this session was
/// established with, because both phases need to send while that reader is receiving.
pub(crate) struct GatewaySession {
    /// Write half. Its counterpart is owned by the reader task for the session's whole life: there is
    /// no reclaim-and-merge here, since a probe registers once and never re-authenticates.
    sink: SplitSink<WsConn, Message>,

    /// The negotiated protocol version, which selects the forward-request variant.
    protocol: GatewayProtocolVersion,

    /// Key the registration handshake derived. Both halves need it: this one to seal what it sends,
    /// the reader to open what arrives.
    shared_key: Arc<SharedSymmetricKey>,

    /// Stops the reader when the session ends.
    shutdown: ShutdownToken,

    /// The reader, awaited on [`close`](Self::close) so a finished probe leaves no task behind.
    reader: JoinHandle<()>,
}

impl GatewaySession {
    /// Connects, negotiates a protocol version, and registers, returning the live session together
    /// with the inbox its reader files onto.
    ///
    /// ALWAYS registers and never authenticates. An unmetered monitor session persists no shared-key
    /// row, so there would be nothing for an authenticate to look up; the cost is one handshake per
    /// run, which suits a process that exits after a single assignment.
    ///
    /// Failure here is the one failure that zeroes BOTH of a run's measurements, so it must be
    /// distinguishable from a phase that merely lost its packets.
    pub(crate) async fn establish(
        target: GatewaySessionTarget,
        identity: &ed25519::KeyPair,
        config: GatewaySessionConfig,
    ) -> anyhow::Result<(Self, GatewaySessionInbox)> {
        // 1. dial the ip and upgrade it to a websocket, with no resolver in the path
        // 2. ask for the gateway's protocol version and settle on one
        // 3. run the client registration handshake, authenticating the gateway's identity
        // 4. split the connection and spawn the reader over the read half
        todo!()
    }

    /// Dials `address` and upgrades the connection, without resolving anything.
    async fn connect(address: SocketAddr, timeout: Duration) -> anyhow::Result<WsConn> {
        todo!()
    }

    /// Asks the gateway which protocol version it speaks and settles on the one to use.
    ///
    /// The policy is the client library's: refuse anything below authenticate-v2 and AES-GCM-SIV, and
    /// clamp a version from the future down to ours. Duplicated here for now; if it is extracted into
    /// `nym-gateway-requests` this becomes a call.
    async fn negotiate_protocol(
        conn: &mut WsConn,
        timeout: Duration,
    ) -> anyhow::Result<GatewayProtocolVersion> {
        todo!()
    }

    /// Performs the registration handshake and returns the key it derived.
    async fn register(
        conn: &mut WsConn,
        target: &GatewaySessionTarget,
        identity: &ed25519::KeyPair,
        protocol: GatewayProtocolVersion,
        timeout: Duration,
    ) -> anyhow::Result<SharedSymmetricKey> {
        todo!()
    }

    /// Hands the gateway one packet to forward, with an explicit next hop.
    ///
    /// The gateway performs no sphinx processing on this path: it forwards the packet verbatim to the
    /// next hop the envelope names. A failure here therefore implicates the session, the bandwidth
    /// path or the outbound forwarder, and never the sphinx layer.
    pub(crate) async fn forward(&mut self, packet: MixPacket) -> anyhow::Result<()> {
        todo!()
    }

    /// Hands the gateway a batch in one flushed write, matching the profile's pacing.
    pub(crate) async fn forward_batch(&mut self, packets: Vec<MixPacket>) -> anyhow::Result<()> {
        todo!()
    }

    /// Seals one forward request under the session key.
    ///
    /// Which request variant carries the packet depends on the negotiated version, since only the
    /// later one carries the sphinx key rotation.
    fn forward_request(&self, packet: MixPacket) -> anyhow::Result<Message> {
        todo!()
    }

    /// Closes the session and stops its reader.
    ///
    /// Held open until BOTH phases and their drain windows are done, never per phase: the delivery
    /// phase needs a live session at the moment its packets reach the gateway, and a session closed
    /// early turns a delivered packet into a dropped one.
    pub(crate) async fn close(self) {
        todo!()
    }
}
