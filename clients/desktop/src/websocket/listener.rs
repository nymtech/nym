// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::handler::Handler;
use log::*;
use std::{
    net::{Shutdown, SocketAddr},
    sync::Arc,
};
use tokio::runtime;
use tokio::{sync::Notify, task::JoinHandle};
use topology::NymTopology;

enum State {
    Connected,
    AwaitingConnection,
}

impl State {
    fn is_connected(&self) -> bool {
        match self {
            State::Connected => true,
            _ => false,
        }
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

    pub(crate) async fn run<T: NymTopology + 'static>(&mut self, handler: Handler<T>) {
        let mut tcp_listener = tokio::net::TcpListener::bind(self.address)
            .await
            .expect("Failed to start websocket listener");

        let notify = Arc::new(Notify::new());

        loop {
            tokio::select! {
                _ = notify.notified() => {
                    // our connection terminated - we are open to a new one now!
                    self.state = State::AwaitingConnection;
                }
                new_conn = tcp_listener.accept() => {
                    match new_conn {
                        Ok((socket, remote_addr)) => {
                            debug!("Received connection from {:?}", remote_addr);
                            if self.state.is_connected() {
                                warn!("tried to duplicate!");
                                // if we've already got a connection, don't allow another one
                                debug!("but there was already a connection present!");
                                // while we only ever want to accept a single connection, we don't want
                                // to leave clients hanging (and also allow for reconnection if it somehow
                                // was dropped)
                                match socket.shutdown(Shutdown::Both) {
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
                                    notify_clone.notify();
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

    pub(crate) fn start<T: NymTopology + 'static>(
        mut self,
        rt_handle: &runtime::Handle,
        handler: Handler<T>,
    ) -> JoinHandle<()> {
        info!("Running websocket on {:?}", self.address.to_string());

        rt_handle.spawn(async move { self.run(handler).await })
    }
}
