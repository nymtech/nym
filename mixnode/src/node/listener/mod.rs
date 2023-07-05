// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::listener::connection_handler::ConnectionHandler;
use quinn::{Endpoint, ServerConfig};
use rcgen::generate_simple_self_signed;
use rustls::{Certificate, PrivateKey};
use std::net::SocketAddr;
use tokio::task::JoinHandle;
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
        let endpoint = Endpoint::server(server_config(), self.address).unwrap();

        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    log::trace!("Listener: Received shutdown");
                },
                connection = endpoint.accept() => {
                    match connection {
                        Some(connecting) => {
                            let conn = connecting.await.unwrap();
                            // TODO: benchmark spawning tokio task with full processing vs just processing it
                            // synchronously (without delaying inside of course,
                            // delay is moved to a global DelayQueue)
                            // under higher load in single and multi-threaded situation.

                            // in theory we could process multiple sphinx packet from the same connection in parallel,
                            // but we already handle multiple concurrent connections so if anything, making
                            // that change would only slow things down
                            debug!("Handling connection from {:?}", conn.remote_address());
                            let handler = connection_handler.clone();
                            tokio::spawn(handler.handle_connection(conn, self.shutdown.clone()));
                        }
                        // Some(Err(err)) => {
                        //     error!(
                        //         "The socket connection got corrupted with error: {err}. Closing the socket",
                        //     );
                        //     return;
                        // }
                        None => {
                            error!("Endpoint closed");
                            break;
                        }, // stream got closed by remote
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

fn generate_self_signed_cert(
) -> Result<(rustls::Certificate, rustls::PrivateKey), Box<dyn std::error::Error>> {
    let cert = generate_simple_self_signed(vec!["mixnode".to_string()])?;
    let key = PrivateKey(cert.serialize_private_key_der());
    Ok((Certificate(cert.serialize_der()?), key))
}
fn server_config() -> ServerConfig {
    let (cert, key) = generate_self_signed_cert().expect("Failed to generate certificate");
    ServerConfig::with_single_cert(vec![cert], key).expect("Failed to generate server config")
}
