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

use crate::node::client_handling::clients_handler::ClientsHandlerRequestSender;
use crate::node::client_handling::websocket::connection_handler::Handle;
use crate::node::mixnet_handling::sender::OutboundMixMessageSender;
use log::*;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

pub(crate) struct Listener {
    address: SocketAddr,
}

impl Listener {
    pub(crate) fn new(address: SocketAddr) -> Self {
        Listener { address }
    }

    pub(crate) async fn run(
        &mut self,
        clients_handler_sender: ClientsHandlerRequestSender,
        outbound_mix_sender: OutboundMixMessageSender,
    ) {
        info!("Starting websocket listener at {}", self.address);
        let mut tcp_listener = tokio::net::TcpListener::bind(self.address)
            .await
            .expect("Failed to start websocket listener");

        loop {
            match tcp_listener.accept().await {
                Ok((socket, remote_addr)) => {
                    trace!("received a socket connection from {}", remote_addr);
                    // TODO: I think we need a mechanism for having a maximum number of connected
                    // clients or spawned tokio tasks -> perhaps a worker system?
                    let mut handle = Handle::new(
                        socket,
                        clients_handler_sender.clone(),
                        outbound_mix_sender.clone(),
                    );
                    tokio::spawn(async move { handle.start_handling().await });
                }
                Err(e) => warn!("failed to get client: {:?}", e),
            }
        }
    }

    pub(crate) fn start(
        mut self,
        clients_handler_sender: ClientsHandlerRequestSender,
        outbound_mix_sender: OutboundMixMessageSender,
    ) -> JoinHandle<()> {
        tokio::spawn(async move { self.run(clients_handler_sender, outbound_mix_sender).await })
    }
}
