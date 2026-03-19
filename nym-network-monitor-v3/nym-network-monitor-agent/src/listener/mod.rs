// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::listener::received::{MixnetPacketsSender, ReceivedPacket};
use futures::StreamExt;
use nym_noise::config::NoiseConfig;
use nym_noise::connection::Connection;
use nym_noise::upgrade_noise_responder;
use nym_sphinx_framing::codec::NymCodec;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;
use tracing::{error, info, warn};

pub(crate) mod received;

/// Listens for inbound sphinx packets returned by the node under test.
///
/// Binds a TCP listener on `bind_address`, accepts a single connection at a time,
/// performs a Noise handshake as the responder, then forwards every decoded
/// [`NymPacket`] to the [`receiver`](received) via `received_packets_sender`.
/// Connections from any address other than `tested_node_address` are rejected.
pub(crate) struct MixnetListener {
    /// Local TCP listener.
    tcp_listener: tokio::net::TcpListener,

    /// Address of the node being tested; connections from any other source are rejected.
    tested_node_address: SocketAddr,

    /// Noise protocol configuration used when upgrading incoming TCP connections.
    noise_config: NoiseConfig,

    /// Channel used to forward received packets to the [`PacketReceiver`](received).
    received_packets_sender: MixnetPacketsSender,

    pub(crate) last_noise_handshake_duration: Option<std::time::Duration>,

    /// Global shutdown token
    shutdown: ShutdownToken,
}

impl MixnetListener {
    /// Creates a new [`MixnetListener`] ready to be started with [`run`](Self::run).
    pub(crate) async fn new(
        bind_address: SocketAddr,
        tested_node_address: SocketAddr,
        noise_config: NoiseConfig,
        received_packets_sender: MixnetPacketsSender,
        shutdown: ShutdownToken,
    ) -> anyhow::Result<Self> {
        info!("attempting to run mixnet listener on {bind_address}");

        let tcp_listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .inspect_err(|err| {
                error!("Failed to the mixnet listener bind to {bind_address}: {err}")
            })?;

        Ok(Self {
            tcp_listener,
            tested_node_address,
            noise_config,
            received_packets_sender,
            last_noise_handshake_duration: None,
            shutdown,
        })
    }

    /// Reads sphinx packets from an established, noise-encrypted stream and forwards
    /// each one to the receiver until the connection is closed or an error occurs.
    async fn handle_stream(&self, mut mixnet_connection: Framed<Connection<TcpStream>, NymCodec>) {
        loop {
            let next_packet = match mixnet_connection.next().await {
                None => {
                    info!("mixnet connection closed");
                    return;
                }
                Some(Ok(packet)) => packet,
                Some(Err(err)) => {
                    error!("failed to read a packet from the mixnet connection: {err}");
                    return;
                }
            };

            if self
                .received_packets_sender
                .unbounded_send(ReceivedPacket::new(next_packet))
                .is_err()
            {
                warn!("mixnet packet receiver has shut down - is the agent still running?");
                return;
            }
        }
    }

    /// Validates the source address, performs the Noise handshake, then delegates to
    /// [`handle_stream`](Self::handle_stream) for the lifetime of the connection.
    async fn handle_connection(&mut self, (socket, source): (TcpStream, SocketAddr)) {
        if source != self.tested_node_address {
            warn!(
                "received a connection from a source that's not the node being tested. Ignoring it. Source: {source}, tested node: {}",
                self.tested_node_address
            );
            return;
        }
        info!("accepted connection from {source}. beginning the noise handshake (responder)");

        let noise_handshake_start = Instant::now();
        let noise_stream = match upgrade_noise_responder(socket, &self.noise_config).await {
            Ok(noise_stream) => noise_stream,
            Err(err) => {
                error!("failed to upgrade the connection to noise with {source}: {err}");
                return;
            }
        };
        let noise_handshake_duration = noise_handshake_start.elapsed();

        if !noise_stream.is_noise() {
            error!(
                "failed to upgrade the connection to noise with {source}. does the node support the protocol?"
            );
            return;
        }
        self.last_noise_handshake_duration = Some(noise_handshake_duration);

        self.handle_stream(Framed::new(noise_stream, NymCodec))
            .await
    }

    /// Binds the TCP listener and processes one connection at a time until the shutdown token is cancelled.
    pub(crate) async fn run(mut self) -> Self {
        // only handle a single connection at once
        // (we don't need more than that)
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    tracing::debug!("mixnet listener: received shutdown");
                    return self
                }
                connection = self.tcp_listener.accept() => {
                    if let Ok(connection) = connection {
                        self.handle_connection(connection).await;
                    } else {
                        error!("failed to accept a TCP connection from the mixnet listener");
                    }
                }
            }
        }
    }
}
