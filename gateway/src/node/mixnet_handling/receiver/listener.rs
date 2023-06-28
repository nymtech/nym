// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::storage::Storage;
use futures::StreamExt;
use log::*;
use nym_sphinx::framing::codec::NymCodec;
use nym_task::TaskClient;
use std::net::SocketAddr;
use std::process;
use tokio::task::JoinHandle;
use tokio_util::udp::UdpFramed;

pub(crate) struct Listener {
    address: SocketAddr,
    shutdown: TaskClient,
}

// TODO: this file is nearly identical to the one in mixnode
impl Listener {
    pub(crate) fn new(address: SocketAddr, shutdown: TaskClient) -> Self {
        Listener { address, shutdown }
    }

    pub(crate) async fn run<St>(&mut self, mut connection_handler: ConnectionHandler<St>)
    where
        St: Storage + Clone + 'static,
    {
        info!("Starting mixnet listener at {}", self.address);
        let socket = match tokio::net::UdpSocket::bind(self.address).await {
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
                    log::trace!("mixnet_handling::Listener: Received shutdown");
                }
                framed_sphinx_packet = framed_conn.next() => {
                    match framed_sphinx_packet {
                        Some(Ok((framed_sphinx_packet, remote))) => {
                            // TODO: benchmark spawning tokio task with full processing vs just processing it
                            // synchronously under higher load in single and multi-threaded situation.

                            // in theory we could process multiple sphinx packet from the same connection in parallel,
                            // but we already handle multiple concurrent connections so if anything, making
                            // that change would only slow things down
                            debug!("Handling packet from {remote:?}");
                            connection_handler.handle_received_packet(framed_sphinx_packet).await;
                        }
                        Some(Err(err)) => {
                            error!(
                                "The socket connection got corrupted with error: {err}. Closing the socket",
                            );
                            return;
                        }
                        None => break, // stream got closed by remote
                    }
                }
            }
        }
    }

    pub(crate) fn start<St>(mut self, connection_handler: ConnectionHandler<St>) -> JoinHandle<()>
    where
        St: Storage + Clone + 'static,
    {
        info!("Running mix listener on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(connection_handler).await })
    }
}
