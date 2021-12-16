// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::handler::Handler;
use log::*;
use std::{net::SocketAddr, process, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::{sync::Notify, task::JoinHandle};

enum State {
    Connected,
    AwaitingConnection,
}

impl State {
    fn is_connected(&self) -> bool {
        matches!(self, State::Connected)
    }
}

pub(crate) struct Listener {
    address: SocketAddr,
    state: State,
}

impl Listener {
    pub(crate) fn new(port: u16) -> Self {
        Listener {
            // unless we find compelling reason not to, just listen on local only
            address: SocketAddr::new("127.0.0.1".parse().unwrap(), port),
            state: State::AwaitingConnection,
        }
    }

    pub(crate) async fn run(&mut self, handler: Handler) {
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address, err);
                process::exit(1);
            }
        };

        let notify = Arc::new(Notify::new());

        loop {
            tokio::select! {
                _ = notify.notified() => {
                    // our connection terminated - we are open to a new one now!
                    self.state = State::AwaitingConnection;
                }
                new_conn = tcp_listener.accept() => {
                    match new_conn {
                        Ok((mut socket, remote_addr)) => {
                            debug!("Received connection from {:?}", remote_addr);
                            if self.state.is_connected() {
                                warn!("tried to duplicate!");
                                // if we've already got a connection, don't allow another one
                                debug!("but there was already a connection present!");
                                // while we only ever want to accept a single connection, we don't want
                                // to leave clients hanging (and also allow for reconnection if it somehow
                                // was dropped)
                                match socket.shutdown().await {
                                    Ok(_) => trace!(
                                        "closed the connection between attempting websocket handshake"
                                    ),
                                    Err(e) => warn!("failed to cleanly close the connection - {:?}", e),
                                };
                            } else {
                                // even though we're spawning a new task with the handler here, we will only ever spawn a single one.
                                // it's done so that any new connections to this listener could be rejected rather than left
                                // hanging because the executor doesn't come back here
                                let notify_clone = Arc::clone(&notify);
                                let fresh_handler = handler.clone();
                                tokio::spawn(async move {
                                    fresh_handler.handle_connection(socket).await;
                                    notify_clone.notify_one();
                                });
                                self.state = State::Connected;
                            }
                        }
                        Err(e) => warn!("failed to get client: {:?}", e),
                    }
                }
            }
        }
    }

    pub(crate) fn start(mut self, handler: Handler) -> JoinHandle<()> {
        info!("Running websocket on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(handler).await })
    }
}
