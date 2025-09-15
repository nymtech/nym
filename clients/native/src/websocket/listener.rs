// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::handler::HandlerBuilder;
use log::*;
use nym_task::TaskClient;
use std::net::IpAddr;
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
    task_client: TaskClient,
}

impl Listener {
    pub(crate) fn new(host: IpAddr, port: u16, task_client: TaskClient) -> Self {
        Listener {
            address: SocketAddr::new(host, port),
            state: State::AwaitingConnection,
            task_client,
        }
    }

    pub(crate) async fn run(&mut self, handler: HandlerBuilder) {
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        let notify = Arc::new(Notify::new());

        while !self.task_client.is_shutdown() {
            tokio::select! {
                // When the handler finishes we check if shutdown is signalled
                _ = notify.notified() => {
                    if self.task_client.is_shutdown() {
                        log::trace!("Websocket listener: detected shutdown after connection closed");
                        break;
                    }
                    // our connection terminated - we are open to a new one now!
                    self.state = State::AwaitingConnection;
                }
                // ... but when there is no connected client at the time of shutdown being
                // signalled, we handle it here.
                _ = self.task_client.recv() => {
                    if !self.state.is_connected() {
                        log::trace!("Not connected: shutting down");
                        break;
                    }
                }
                new_conn = tcp_listener.accept() => {
                    match new_conn {
                        Ok((mut socket, remote_addr)) => {
                            debug!("Received connection from {remote_addr:?}");
                            if self.state.is_connected() {
                                warn!("Tried to open a duplicate websocket connection. The request came from {remote_addr}");
                                // if we've already got a connection, don't allow another one
                                // while we only ever want to accept a single connection, we don't want
                                // to leave clients hanging (and also allow for reconnection if it somehow
                                // was dropped)
                                match socket.shutdown().await {
                                    Ok(_) => trace!(
                                        "closed the connection between attempting websocket handshake"
                                    ),
                                    Err(err) => warn!("failed to cleanly close the connection - {err}"),
                                };
                            } else {
                                // even though we're spawning a new task with the handler here, we will only ever spawn a single one.
                                // it's done so that any new connections to this listener could be rejected rather than left
                                // hanging because the executor doesn't come back here
                                let notify_clone = Arc::clone(&notify);
                                let fresh_handler = handler.create_active_handler();
                                tokio::spawn(async move {
                                    fresh_handler.handle_connection(socket).await;
                                    notify_clone.notify_one();
                                });
                                self.state = State::Connected;
                            }
                        }
                        Err(err) => warn!("failed to get client: {err}"),
                    }
                }
            }
        }
        log::debug!("Websocket listener: Exiting");
    }

    pub(crate) fn start(mut self, handler: HandlerBuilder) -> JoinHandle<()> {
        info!("Running websocket on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(handler).await })
    }
}
