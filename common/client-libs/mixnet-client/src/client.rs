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

use crate::connection_manager::{ConnectionManager, ConnectionManagerSender};
use futures::channel::oneshot;
use futures::future::AbortHandle;
use log::*;
use nymsphinx::framing::packet::FramedSphinxPacket;
use nymsphinx::params::PacketMode;
use nymsphinx::{addressing::nodes::NymNodeRoutingAddress, SphinxPacket};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

pub struct Config {
    initial_reconnection_backoff: Duration,
    maximum_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
        }
    }
}

pub struct Client {
    connections_managers: HashMap<SocketAddr, (ConnectionManagerSender, AbortHandle)>,
    maximum_reconnection_backoff: Duration,
    initial_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            connections_managers: HashMap::new(),
            initial_reconnection_backoff: config.initial_reconnection_backoff,
            maximum_reconnection_backoff: config.maximum_reconnection_backoff,
            initial_connection_timeout: config.initial_connection_timeout,
        }
    }

    async fn start_new_connection_manager(
        &mut self,
        address: SocketAddr,
    ) -> (ConnectionManagerSender, AbortHandle) {
        let (sender, abort_handle) = ConnectionManager::new(
            address,
            self.initial_reconnection_backoff,
            self.maximum_reconnection_backoff,
            self.initial_connection_timeout,
        )
        .await
        .spawn_abortable();

        (sender, abort_handle)
    }

    // if wait_for_response is set to true, we will get information about any possible IO errors
    // as well as (once implemented) received replies, however, this will also cause way longer
    // waiting periods
    pub async fn send(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: SphinxPacket,
        packet_mode: PacketMode,
        wait_for_response: bool,
    ) -> io::Result<()> {
        trace!("Sending packet to {:?}", address);
        let socket_address = address.into();

        if !self.connections_managers.contains_key(&socket_address) {
            debug!(
                "There is no existing connection to {:?} - it will be established now",
                address
            );

            let (new_manager_sender, abort_handle) =
                self.start_new_connection_manager(socket_address).await;
            self.connections_managers
                .insert(socket_address, (new_manager_sender, abort_handle));
        }

        let manager = self.connections_managers.get_mut(&socket_address).unwrap();

        let framed_packet = FramedSphinxPacket::new(packet, packet_mode);

        if wait_for_response {
            let (res_tx, res_rx) = oneshot::channel();
            manager
                .0
                .unbounded_send((framed_packet, Some(res_tx)))
                .unwrap();
            res_rx.await.unwrap()
        } else {
            manager.0.unbounded_send((framed_packet, None)).unwrap();
            Ok(())
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        for (_, abort_handle) in self.connections_managers.values() {
            abort_handle.abort()
        }
    }
}
