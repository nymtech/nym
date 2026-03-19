// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use futures::{SinkExt, stream};
use nym_noise::config::NoiseConfig;
use nym_noise::connection::Connection;
use nym_noise::upgrade_noise_initiator;
use nym_sphinx_framing::codec::NymCodec;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_params::{PacketType, SphinxKeyRotation};
use nym_sphinx_types::{NymPacket, SphinxPacket};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout};
use tokio_util::codec::Framed;
use tracing::{error, info};

/// Timing statistics collected over the lifetime of an [`EgressConnection`].
pub(crate) struct EgressConnectionStatistics {
    /// Duration of the Noise handshake performed when the connection was established.
    pub(crate) noise_handshake_duration: std::time::Duration,

    /// Per-batch send durations, one entry for each call to [`send_packet_batch`](EgressConnection::send_packet_batch).
    pub(crate) packet_batches_sending_duration: Vec<std::time::Duration>,
}

/// An outbound, noise-encrypted TCP connection to the node under test used for sending sphinx packets.
pub(crate) struct EgressConnection {
    /// Timing statistics accumulated while the connection is active.
    pub(crate) connection_statistics: EgressConnectionStatistics,

    /// The key rotation at the time of starting the agent.
    key_rotation: SphinxKeyRotation,

    /// The noise-encrypted, framed TCP stream used to send sphinx packets.
    mixnet_connection: Framed<Connection<TcpStream>, NymCodec>,
}

impl EgressConnection {
    /// Opens a TCP connection to `address`, performs the Noise handshake as the initiator,
    /// and returns a ready-to-use [`EgressConnection`].
    /// Fails if the TCP connect or Noise upgrade exceeds timeout.
    pub(crate) async fn establish(
        address: SocketAddr,
        timeout_duration: std::time::Duration,
        key_rotation: SphinxKeyRotation,
        noise_config: &NoiseConfig,
    ) -> anyhow::Result<Self> {
        info!("attempting to establish connection to {address}");
        let stream = timeout(timeout_duration, TcpStream::connect(address)).await??;

        info!("beginning the noise handshake (initiator)");

        let noise_handshake_start = Instant::now();
        let noise_stream = upgrade_noise_initiator(stream, noise_config).await?;

        if !noise_stream.is_noise() {
            error!(
                "failed to upgrade the connection to noise with {address}. does the node support the protocol?"
            );
            bail!("egress connection failure");
        }

        let noise_handshake_duration = noise_handshake_start.elapsed();

        Ok(Self {
            connection_statistics: EgressConnectionStatistics {
                noise_handshake_duration,
                packet_batches_sending_duration: vec![],
            },
            key_rotation,
            mixnet_connection: Framed::new(noise_stream, NymCodec),
        })
    }

    /// Sends a single sphinx packet and records the send duration in [`EgressConnectionStatistics`].
    pub(crate) async fn send_packet(&mut self, packet: SphinxPacket) -> anyhow::Result<()> {
        self.mixnet_connection
            .send(FramedNymPacket::new(
                NymPacket::Sphinx(packet),
                PacketType::Mix,
                self.key_rotation,
                false,
            ))
            .await?;

        Ok(())
    }

    /// Sends a batch of sphinx packets in one flushed write and records the total batch send duration.
    pub(crate) async fn send_packet_batch(
        &mut self,
        packets: Vec<SphinxPacket>,
    ) -> anyhow::Result<()> {
        let send_start = Instant::now();
        self.mixnet_connection
            .send_all(&mut stream::iter(packets.into_iter().map(|p| {
                Ok(FramedNymPacket::new(
                    NymPacket::Sphinx(p),
                    PacketType::Mix,
                    self.key_rotation,
                    false,
                ))
            })))
            .await?;
        self.connection_statistics
            .packet_batches_sending_duration
            .push(send_start.elapsed());
        Ok(())
    }
}
