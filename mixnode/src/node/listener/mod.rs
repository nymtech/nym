// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::listener::connection_handler::ConnectionHandler;
use futures::StreamExt;
use nym_sphinx::framing::codec::NymCodec;
use std::net::SocketAddr;
use std::process;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tokio_util::udp::UdpFramed;
#[cfg(feature = "cpucycles")]
use tracing::error;

use super::TaskClient;

pub(crate) mod connection_handler;

pub(crate) struct Listener {
    address: SocketAddr,
    shutdown: TaskClient,
}

impl Listener {
    pub(crate) fn new(address: SocketAddr, shutdown: TaskClient) -> Self {
        Listener { address, shutdown }
    }

    async fn run(&mut self, connection_handler: ConnectionHandler) {
        log::trace!("Starting Listener");
        let socket = match UdpSocket::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        let mut framed_conn = UdpFramed::new(socket, NymCodec);

        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    log::trace!("Listener: Received shutdown");
                },
                framed_sphinx_packet = framed_conn.next() => {
                    match framed_sphinx_packet {
                        Some(Ok((framed_sphinx_packet, remote))) => {
                            // TODO: benchmark spawning tokio task with full processing vs just processing it
                            // synchronously (without delaying inside of course,
                            // delay is moved to a global DelayQueue)
                            // under higher load in single and multi-threaded situation.

                            // in theory we could process multiple sphinx packet from the same connection in parallel,
                            // but we already handle multiple concurrent connections so if anything, making
                            // that change would only slow things down
                            debug!("Handling packet from {remote:?}");
                            connection_handler.handle_received_packet(framed_sphinx_packet);
                        }
                        Some(Err(err)) => {
                            error!(
                                "The socket connection got corrupted with error: {err}. Closing the socket",
                            );
                            return;
                        }
                        None => break, // stream got closed by remote
                    }
                },
            };
        }
        log::trace!("Listener: Exiting");
    }

    pub(crate) fn start(mut self, connection_handler: ConnectionHandler) -> JoinHandle<()> {
        info!("Running mix listener on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(connection_handler).await })
    }
}
