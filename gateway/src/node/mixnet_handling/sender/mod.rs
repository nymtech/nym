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

// TODO: code is nearly identical to mixnode::node::packet_forwarding -> perhaps it should be put to common?

use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::task::JoinHandle;

pub(crate) type OutboundMixMessageSender = mpsc::UnboundedSender<(SocketAddr, Vec<u8>)>;
pub(crate) type OutboundMixMessageReceiver = mpsc::UnboundedReceiver<(SocketAddr, Vec<u8>)>;

pub(crate) struct PacketForwarder {
    tcp_client: multi_tcp_client::Client,
    conn_tx: OutboundMixMessageSender,
    conn_rx: OutboundMixMessageReceiver,
}

impl PacketForwarder {
    pub(crate) fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
    ) -> PacketForwarder {
        let tcp_client_config = multi_tcp_client::Config::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
        );

        let (conn_tx, conn_rx) = mpsc::unbounded();

        PacketForwarder {
            tcp_client: multi_tcp_client::Client::new(tcp_client_config),
            conn_tx,
            conn_rx,
        }
    }

    pub(crate) fn start(mut self) -> (JoinHandle<()>, OutboundMixMessageSender) {
        let sender_channel = self.conn_tx.clone();
        (
            tokio::spawn(async move {
                while let Some((address, packet)) = self.conn_rx.next().await {
                    trace!("Going to forward packet to {:?}", address);
                    // as a mix node we don't care about responses, we just want to fire packets
                    // as quickly as possible
                    self.tcp_client.send(address, packet, false).await.unwrap();
                    // if we're not waiting for response, we MUST get an Ok
                }
            }),
            sender_channel,
        )
    }
}
