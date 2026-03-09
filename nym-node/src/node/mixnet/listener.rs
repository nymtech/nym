// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::SharedData;
use crate::node::mixnet::packet_ingest::{IngressNymPacket, MixIngestSender};
use futures::StreamExt;
use nym_noise::connection::Connection;
use nym_noise::upgrade_noise_responder;
use nym_sphinx_framing::codec::NymCodec;
use nym_task::ShutdownToken;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{Span, debug, error, info, instrument, trace, warn};

use std::io;
use std::net::SocketAddr;
use std::time::Instant;

/// How often (in packets) the stream-level span updates its packet count.
const SPAN_UPDATE_INTERVAL: u64 = 10_000;

pub(crate) struct Listener {
    bind_address: SocketAddr,
    shared_data: SharedData,
    ingest_sender: MixIngestSender,
}

impl Listener {
    pub(crate) fn new(
        bind_address: SocketAddr,
        shared_data: SharedData,
        ingest_sender: MixIngestSender,
    ) -> Self {
        Listener {
            bind_address,
            shared_data,
            ingest_sender,
        }
    }

    pub(crate) async fn run(&mut self, shutdown: ShutdownToken) {
        info!("attempting to run mixnet listener on {}", self.bind_address);

        let tcp_listener = match tokio::net::TcpListener::bind(self.bind_address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!(
                    "Failed to bind to {}: {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?",
                    self.bind_address
                );
                shutdown.cancel();
                return;
            }
        };

        loop {
            tokio::select! {
                biased;
                _ = shutdown.cancelled() => {
                    trace!("mixnet listener: received shutdown");
                    break
                }
                connection = tcp_listener.accept() => {
                    self.try_handle_connection(connection);
                }
            }
        }
        debug!("mixnet socket listener: Exiting");
    }

    fn try_handle_connection(
        &self,
        accepted: io::Result<(TcpStream, SocketAddr)>,
    ) -> Option<JoinHandle<()>> {
        match accepted {
            Ok((socket, remote_addr)) => {
                debug!("accepted incoming mixnet connection from: {remote_addr}");

                let conn_handler = ConnectionHandler::new(
                    &self.shared_data,
                    remote_addr,
                    self.ingest_sender.clone(),
                );
                let join_handle =
                    tokio::spawn(async move { conn_handler.handle_connection(socket).await });
                self.shared_data.log_connected_clients();
                Some(join_handle)
            }
            Err(err) => {
                debug!("failed to accept incoming mixnet connection: {err}");
                None
            }
        }
    }
}

struct ConnectionHandler {
    remote_addr: SocketAddr,
    shared_data: SharedData,
    ingest_sender: MixIngestSender,
}

impl ConnectionHandler {
    fn new(shared: &SharedData, remote_addr: SocketAddr, ingest_sender: MixIngestSender) -> Self {
        Self {
            ingest_sender,
            remote_addr,
            shared_data: SharedData {
                processing_config: shared.processing_config,
                sphinx_keys: shared.sphinx_keys.clone(),
                replay_protection_filter: shared.replay_protection_filter.clone(),
                mixnet_forwarder: shared.mixnet_forwarder.clone(),
                final_hop: shared.final_hop.clone(),
                noise_config: shared.noise_config.clone(),
                metrics: shared.metrics.clone(),
                shutdown_token: shared.shutdown_token.child_token(),
            },
        }
    }

    #[instrument(
        name = "mixnode.connection",
        skip(self, socket),
        level = "debug",
        fields(
            remote = %self.remote_addr,
            noise_handshake_ms = tracing::field::Empty,
        )
    )]
    pub(crate) async fn handle_connection(&self, socket: TcpStream) {
        let handshake_start = Instant::now();
        let noise_stream =
            match upgrade_noise_responder(socket, &self.shared_data.noise_config).await {
                Ok(noise_stream) => noise_stream,
                Err(err) => {
                    Span::current().record(
                        "noise_handshake_ms",
                        handshake_start.elapsed().as_millis() as u64,
                    );
                    warn!(
                        event = "connection.failed.noise",
                        remote_addr = %self.remote_addr,
                        error = %err,
                        "Noise responder handshake failed"
                    );
                    return;
                }
            };
        Span::current().record(
            "noise_handshake_ms",
            handshake_start.elapsed().as_millis() as u64,
        );
        debug!(
            "Noise responder handshake completed for {:?}",
            self.remote_addr
        );
        self.handle_stream(Framed::new(noise_stream, NymCodec))
            .await
    }

    #[instrument(
        name = "mixnode.stream",
        skip(self, mixnet_connection),
        level = "debug",
        fields(
            remote = %self.remote_addr,
            packets_processed = 0u64,
            exit_reason,
        )
    )]
    pub(crate) async fn handle_stream(
        &self,
        mut mixnet_connection: Framed<Connection<TcpStream>, NymCodec>,
    ) {
        let mut packets_processed: u64 = 0;
        loop {
            tokio::select! {
                biased;
                _ = self.shared_data.shutdown_token.cancelled() => {
                    trace!("connection handler: received shutdown");
                    Span::current().record("exit_reason", "shutdown");
                    break
                }
                maybe_framed_nym_packet = mixnet_connection.next() => {
                    match maybe_framed_nym_packet {
                        Some(Ok(packet)) => {
                            let ingress_packet = IngressNymPacket::new(packet, Instant::now(), self.remote_addr);
                            self.handle_received_nym_packet(ingress_packet);
                            packets_processed += 1;
                            if packets_processed.is_multiple_of(SPAN_UPDATE_INTERVAL) {
                                Span::current().record("packets_processed", packets_processed);
                            }
                        }
                        Some(Err(err)) => {
                            warn!(
                                event = "connection.corrupted",
                                remote_addr = %self.remote_addr,
                                error = %err,
                                packets_processed,
                                "connection stream corrupted"
                            );
                            Span::current().record("exit_reason", "corrupted");
                            Span::current().record("packets_processed", packets_processed);
                            return
                        }
                        None => {
                            debug!(
                                remote_addr = %self.remote_addr,
                                packets_processed,
                                "connection closed by remote"
                            );
                            Span::current().record("exit_reason", "closed_by_remote");
                            Span::current().record("packets_processed", packets_processed);
                            return
                        }
                    }
                }
            }
        }

        Span::current().record("packets_processed", packets_processed);
        debug!("exiting and closing connection");
    }

    /// Attempt to add the packet to the processing queue. If there is no capacity available the packet will
    /// be dropped and the `mixnet_ingress_overflow_packets_dropped` metric will be incremented.
    fn handle_received_nym_packet(&self, packet: IngressNymPacket) {
        match self.ingest_sender.ingest_packet(packet) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => self
                .shared_data
                .metrics
                .mixnet
                .ingress_dropped_overflow_packet(),
            Err(err) => {
                error!("unexpected error using ingress channel - shutting down: {err}");
                self.shared_data.shutdown_token.cancel();
            }
        }
    }
}
