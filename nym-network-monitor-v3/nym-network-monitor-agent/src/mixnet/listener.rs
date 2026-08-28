// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::demux::{UpgradedConnection, UpgradedConnectionsSender};
use crate::mixnet::targets::WaveIngress;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::NoiseConfig;
use nym_noise::connection::Connection;
use nym_noise::upgrade_noise_responder;
use nym_sphinx_framing::codec::NymCodec;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio_util::codec::Framed;
use tracing::{error, info, warn};

/// A mixnet connection once its Noise handshake has completed: the production instantiation of the
/// connection type [`IngressDemux`](crate::mixnet::demux::IngressDemux) polls.
pub(crate) type NoiseMixnetConnection = Framed<Connection<TcpStream>, NymCodec>;

/// Accepts the connections on which a wave's targets return their test packets.
///
/// ONE listener serves every target of a wave. It does no reading of its own: each accepted
/// connection is gated on being one of the wave's targets, upgraded to Noise in a task of its own,
/// and handed to the demux, which is what polls it and attributes what arrives.
///
/// The per-connection task is not an optimisation. `upgrade_noise_responder` is async, so upgrading
/// inline would let one slow or unresponsive peer delay accepting and upgrading every other target
/// of the wave, which is the serialisation a shared listener exists to remove.
pub(crate) struct MixnetListener {
    /// Local TCP listener.
    tcp_listener: tokio::net::TcpListener,

    /// The wave's targets. Resolving a connection against them serves two purposes: refusing a
    /// source no target is known by, and supplying the key for that connection's responder config.
    targets: Arc<WaveIngress>,

    /// The agent's own Noise key pair, which is what the responder handshake authenticates with.
    noise_key: Arc<x25519::KeyPair>,

    /// Timeout applied to each connection's Noise handshake.
    noise_handshake_timeout: Duration,

    /// Where upgraded connections are handed over to be polled.
    upgrades: UpgradedConnectionsSender<NoiseMixnetConnection>,

    /// Global shutdown token
    shutdown: ShutdownToken,
}

impl MixnetListener {
    /// Binds the listener, ready to be started with [`run`](Self::run).
    pub(crate) async fn new(
        bind_address: SocketAddr,
        targets: Arc<WaveIngress>,
        noise_key: Arc<x25519::KeyPair>,
        noise_handshake_timeout: Duration,
        upgrades: UpgradedConnectionsSender<NoiseMixnetConnection>,
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
            targets,
            noise_key,
            noise_handshake_timeout,
            upgrades,
            shutdown,
        })
    }

    /// Accepts connections until the shutdown token is cancelled.
    ///
    /// Returning drops this listener's half of the upgrades channel, which is what lets the demux
    /// finish once it has drained the connections it already holds.
    pub(crate) async fn run(self, on_start: Arc<Notify>) {
        on_start.notify_one();

        // held so that a handshake still in flight when the wave ends is aborted rather than
        // outliving it
        let mut handshakes = JoinSet::new();

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    tracing::debug!("mixnet listener: received shutdown");
                    handshakes.abort_all();
                    return;
                }
                accepted = self.tcp_listener.accept() => {
                    let Ok((socket, source)) = accepted else {
                        error!("failed to accept a TCP connection from the mixnet listener");
                        continue;
                    };
                    self.begin_handshake(socket, source, &mut handshakes);
                }
            }
        }
    }

    /// Resolves an accepted connection to its target and starts upgrading it.
    ///
    /// The source is checked here, BEFORE the handshake, so that a stranger hitting the port cannot
    /// consume one. Because the check happens first, the handshake's outcome is also attributable to
    /// a specific target rather than being an anonymous log line.
    fn begin_handshake(&self, socket: TcpStream, source: SocketAddr, handshakes: &mut JoinSet<()>) {
        let Some(target) = self.targets.target(source.ip()) else {
            warn!(
                "received a connection from {source}, which is not a target of this wave. ignoring it"
            );
            return;
        };

        info!("accepted connection from {source}. beginning the noise handshake (responder)");
        let noise_config = target.responder_config(
            source.ip(),
            self.noise_key.clone(),
            self.noise_handshake_timeout,
        );
        let upgrades = self.upgrades.clone();

        handshakes.spawn(upgrade_connection(socket, source, noise_config, upgrades));
    }
}

/// Performs the responder handshake for one connection and reports the outcome to the demux.
///
/// Both failure modes are reported rather than logged and dropped, and they are kept distinct: a
/// handshake that errors is a different diagnosis from one that "succeeded" into a plain TCP
/// connection, which is what the Noise responder does when it does not recognise the source.
async fn upgrade_connection(
    socket: TcpStream,
    source: SocketAddr,
    noise_config: NoiseConfig,
    upgrades: UpgradedConnectionsSender<NoiseMixnetConnection>,
) {
    let handshake_start = Instant::now();

    let outcome = match upgrade_noise_responder(socket, &noise_config).await {
        Ok(stream) if stream.is_noise() => UpgradedConnection::Ready {
            source: source.ip(),
            handshake: handshake_start.elapsed(),
            stream: Framed::new(stream, NymCodec),
        },
        Ok(_) => UpgradedConnection::Failed {
            source: source.ip(),
            error: format!(
                "the connection from {source} was not upgraded to noise. does the node support the protocol?"
            ),
        },
        Err(err) => UpgradedConnection::Failed {
            source: source.ip(),
            error: format!("failed to upgrade the connection from {source} to noise: {err}"),
        },
    };

    if upgrades.unbounded_send(outcome).is_err() {
        warn!("the mixnet demux has shut down - is the agent still running?");
    }
}
