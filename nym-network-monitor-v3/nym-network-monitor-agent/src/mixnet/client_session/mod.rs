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

use crate::mixnet::client_session::inbox::GatewaySessionInbox;
use crate::mixnet::client_session::reader::SessionReader;
use anyhow::{Context, anyhow, bail};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt, stream};
use nym_crypto::asymmetric::ed25519;
use nym_gateway_requests::registration::handshake::{HandshakeResult, client_handshake};
use nym_gateway_requests::{
    BinaryRequest, CURRENT_PROTOCOL_VERSION, ClientControlRequest, GatewayProtocolVersion,
    GatewayProtocolVersionExt, ServerResponse, SharedSymmetricKey,
};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_task::ShutdownToken;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::pin;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::client_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

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
#[derive(Debug, Clone, Copy)]
pub(crate) struct GatewaySessionConfig {
    /// Bounds the TCP connect and the websocket upgrade.
    pub(crate) connect_timeout: Duration,

    /// Bounds protocol negotiation and the registration handshake.
    pub(crate) registration_timeout: Duration,

    /// How long the inbox waits for a single delivered payload.
    pub(crate) receive_timeout: Duration,
}

/// A connection to a gateway's client websocket that has not registered yet.
///
/// Owns the UNSPLIT connection through establishment, so the steps read as a sequence of methods on
/// one value rather than as free functions each threading `&mut WsConn` and a timeout of its own.
/// [`split`](Self::split) consumes it, and there is no way back: that is what makes "a probe
/// registers once and never re-authenticates" structural rather than a convention, and it is why
/// there is no reclaim-and-merge of the two halves anywhere in this module.
struct UnregisteredSession {
    conn: WsConn,

    /// The timeouts every step below is held to.
    config: GatewaySessionConfig,

    /// Cancels establishment when the agent is shutting down, so a gateway that accepts a connection
    /// and then goes quiet cannot outlive the process by the length of a timeout.
    shutdown: ShutdownToken,
}

impl UnregisteredSession {
    /// Dials `address` and upgrades the connection, without resolving anything.
    async fn connect(
        address: SocketAddr,
        config: GatewaySessionConfig,
        shutdown: ShutdownToken,
    ) -> anyhow::Result<Self> {
        // built from the ip we were assigned, so nothing here consults a resolver. `SocketAddr`'s
        // display already brackets an ipv6 address, which is what the url form needs
        let endpoint = format!("ws://{address}");
        debug!("dialling {endpoint}");

        let stream = timeout(config.connect_timeout, TcpStream::connect(address))
            .await
            .with_context(|| format!("timed out connecting to {endpoint}"))?
            .with_context(|| format!("failed to connect to {endpoint}"))?;

        let (conn, _) = timeout(config.connect_timeout, client_async(&endpoint, stream))
            .await
            .with_context(|| format!("timed out upgrading {endpoint} to a websocket"))?
            .with_context(|| format!("failed to upgrade {endpoint} to a websocket"))?;

        Ok(UnregisteredSession {
            conn,
            config,
            shutdown,
        })
    }

    /// Asks the gateway which protocol version it speaks and settles on the one to propose.
    ///
    /// The policy is the client library's: refuse anything below authenticate-v2 and AES-GCM-SIV, and
    /// clamp a version from the future down to ours. Duplicated here for now; if it is extracted into
    /// `nym-gateway-requests` this becomes a call.
    async fn negotiate_protocol(&mut self) -> anyhow::Result<GatewayProtocolVersion> {
        self.conn
            .send(Message::from(ClientControlRequest::SupportedProtocol {}))
            .await
            .context("failed to ask the gateway for its protocol version")?;

        let announced = match self.read_control_response().await? {
            ServerResponse::SupportedProtocol { version } => version,
            ServerResponse::Error { message } => {
                bail!("the gateway refused to report its protocol version: {message}")
            }
            other => bail!(
                "expected the gateway's protocol version, got a {} response",
                other.name()
            ),
        };

        if !announced.supports_authenticate_v2() || !announced.supports_aes256_gcm_siv() {
            bail!(
                "the gateway announced protocol v{announced}, which predates authentication v2 or AES256-GCM-SIV and is no longer supported"
            )
        }

        if announced.is_future_version() {
            warn!(
                "the gateway announced protocol v{announced} while we speak v{CURRENT_PROTOCOL_VERSION}; attempting to downgrade"
            );
            return Ok(CURRENT_PROTOCOL_VERSION);
        }

        Ok(announced)
    }

    /// Performs the registration handshake and returns what it settled on.
    ///
    /// ALWAYS registers, and the identity it presents is the ANNOUNCED one: the gateway's exemption
    /// is a membership test over the identities the contract holds, so a freshly generated key would
    /// register perfectly well and then be metered.
    async fn register(
        &mut self,
        target: &GatewaySessionTarget,
        identity: &ed25519::KeyPair,
        proposed_protocol: GatewayProtocolVersion,
    ) -> anyhow::Result<HandshakeResult> {
        let mut rng = OsRng;

        let handshake = timeout(
            self.config.registration_timeout,
            client_handshake(
                &mut rng,
                &mut self.conn,
                identity,
                target.identity,
                proposed_protocol,
                self.shutdown.clone(),
            ),
        )
        .await
        .context("timed out on the registration handshake")?
        .context("the registration handshake with the gateway failed")?;

        match self.read_control_response().await? {
            ServerResponse::Register { status: true, .. } => Ok(handshake),
            ServerResponse::Register { status: false, .. } => {
                bail!("the gateway rejected our registration")
            }
            ServerResponse::Error { message } => {
                bail!("the gateway refused our registration: {message}")
            }
            other => bail!(
                "expected the outcome of our registration, got a {} response",
                other.name()
            ),
        }
    }

    /// Reads the next control response, ignoring frames that are not one.
    ///
    /// Only used during establishment, before the session carries any traffic. Once it does, control
    /// frames are the reader's business.
    ///
    /// Cancellable as well as bounded: this is where a gateway that accepts a connection and then
    /// says nothing is waited on, so honouring only the deadline would keep a whole wave of sessions
    /// alive for its length after the agent had been told to stop.
    async fn read_control_response(&mut self) -> anyhow::Result<ServerResponse> {
        // cloned rather than borrowed off `self`, which leaves `self.conn` free for the mutable
        // borrow the read needs. a clone shares the original's cancellation state, unlike a child
        // token, so this still fires on a global shutdown
        let shutdown = self.shutdown.clone();

        // created ONCE, outside the loop: bounding each read rather than the whole wait would let a
        // gateway sending non-text frames extend it indefinitely
        let deadline = sleep(self.config.registration_timeout);
        pin!(deadline);

        loop {
            let message = tokio::select! {
                _ = &mut deadline => {
                    bail!("timed out awaiting a control response from the gateway")
                }
                _ = shutdown.cancelled() => bail!("the agent is shutting down"),
                message = self.conn.next() => message,
            };

            let message = message
                .context("the gateway closed the connection")?
                .context("failed to read from the gateway")?;

            match message {
                Message::Text(text) => {
                    return ServerResponse::try_from(text).map_err(|err| {
                        anyhow!("the gateway sent a control frame that did not parse: {err}")
                    });
                }
                other => debug!(
                    "ignoring a non-text frame of {} byte(s) while awaiting a control response",
                    other.len()
                ),
            }
        }
    }

    /// Splits the registered connection into the halves a live session is made of.
    fn split(self) -> (SplitSink<WsConn, Message>, SplitStream<WsConn>) {
        self.conn.split()
    }
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

    /// Stops the reader when the session ends. A CHILD of the agent's token, so closing one session
    /// leaves its siblings running while a cancel from above still reaches it.
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
    ///
    /// `shutdown` is the agent's, not a fresh one: cancelling it has to reach this session's reader
    /// and its establishment, so that a wave of sessions dies with the process rather than each
    /// holding a task nothing can reach.
    pub(crate) async fn establish(
        target: GatewaySessionTarget,
        identity: &ed25519::KeyPair,
        config: GatewaySessionConfig,
        shutdown: ShutdownToken,
    ) -> anyhow::Result<(Self, GatewaySessionInbox)> {
        info!(
            "establishing a client session with {} at {}",
            target.identity, target.address
        );

        // 1. dial the ip and upgrade it to a websocket, with no resolver in the path
        let mut pending =
            UnregisteredSession::connect(target.address, config, shutdown.clone()).await?;

        // 2. ask for the gateway's protocol version and settle on one to propose
        let proposed = pending.negotiate_protocol().await?;

        // 3. register, which authenticates the gateway against the identity the contract holds for
        // it and proves ours to the gateway in the same exchange
        let handshake = pending.register(&target, identity, proposed).await?;

        // the handshake may settle on a version below the one we proposed, and it is the settled one
        // that decides which forward request carries a packet
        let protocol = handshake.negotiated_protocol;
        let shared_key = Arc::new(handshake.derived_key);
        debug!(
            "registered with {} on protocol v{protocol}",
            target.identity
        );

        // 4. split the connection: the read half belongs to a spawned reader for the session's whole
        // life, because both phases need to send while it receives.
        //
        // the reader gets a CHILD token so that closing one session stops only its own reader, while
        // a cancel from above still reaches every one of them
        let inbox = GatewaySessionInbox::new(config.receive_timeout);
        let reader_shutdown = shutdown.child_token();
        let (sink, read_half) = pending.split();
        let reader = SessionReader::new(shared_key.clone(), inbox.events_sender())
            .spawn(read_half, reader_shutdown.clone());

        Ok((
            GatewaySession {
                sink,
                protocol,
                shared_key,
                shutdown: reader_shutdown,
                reader,
            },
            inbox,
        ))
    }

    /// Hands the gateway a batch of packets to forward, each with an explicit next hop, in one
    /// flushed write.
    ///
    /// The gateway performs no sphinx processing on this path: it forwards each packet verbatim to the
    /// next hop its envelope names. A failure here therefore implicates the session, the bandwidth
    /// path or the outbound forwarder, and never the sphinx layer.
    ///
    /// Only the batched form exists: a probe always sends on its profile's pacing, so a single-packet
    /// variant would be a batch of one under another name.
    pub(crate) async fn forward_batch(&mut self, packets: Vec<MixPacket>) -> anyhow::Result<()> {
        let requests = packets
            .into_iter()
            .map(|packet| self.forward_request(packet))
            .collect::<anyhow::Result<Vec<_>>>()?;

        self.sink
            .send_all(&mut stream::iter(requests.into_iter().map(Ok)))
            .await
            .context("failed to hand a batch of packets to the gateway")?;
        Ok(())
    }

    /// Seals one forward request under the session key.
    ///
    /// Which request variant carries the packet depends on the negotiated version, since only the
    /// later one carries the sphinx key rotation.
    fn forward_request(&self, packet: MixPacket) -> anyhow::Result<Message> {
        let request = if self.protocol.supports_key_rotation_packet() {
            BinaryRequest::ForwardSphinxV2 { packet }
        } else {
            BinaryRequest::ForwardSphinx { packet }
        };

        request
            .into_ws_message(&self.shared_key)
            .context("failed to seal a forward request for the gateway")
    }

    /// Closes the session and stops its reader.
    ///
    /// Held open until BOTH phases and their drain windows are done, never per phase: the delivery
    /// phase needs a live session at the moment its packets reach the gateway, and a session closed
    /// early turns a delivered packet into a dropped one.
    pub(crate) async fn close(mut self) {
        // best effort: a gateway that has already hung up is the ordinary end of a run rather than a
        // failure of it, and there is nothing left to measure either way
        if let Err(err) = self.sink.close().await {
            debug!("the session's write half did not close cleanly: {err}");
        }

        // cancels only this session's reader, since the token is a child of the agent's
        self.shutdown.cancel();
        if let Err(err) = self.reader.await {
            warn!("the session reader did not shut down cleanly: {err}");
        }
    }
}
