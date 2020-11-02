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
    maximum_reconnection_attempts: u32,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_reconnection_attempts: u32,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_reconnection_attempts,
        }
    }
}

pub struct Client {
    connections_managers: HashMap<SocketAddr, (ConnectionManagerSender, AbortHandle)>,
    maximum_reconnection_backoff: Duration,
    initial_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
    maximum_reconnection_attempts: u32,
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            connections_managers: HashMap::new(),
            initial_reconnection_backoff: config.initial_reconnection_backoff,
            maximum_reconnection_backoff: config.maximum_reconnection_backoff,
            initial_connection_timeout: config.initial_connection_timeout,
            maximum_reconnection_attempts: config.maximum_reconnection_attempts,
        }
    }

    async fn start_new_connection_manager(
        &mut self,
        address: SocketAddr,
    ) -> Result<(ConnectionManagerSender, AbortHandle), io::Error> {
        let conn_manager = ConnectionManager::new(
            address,
            self.initial_reconnection_backoff,
            self.maximum_reconnection_backoff,
            self.initial_connection_timeout,
            self.maximum_reconnection_attempts,
        )
        .await?;

        let (sender, abort_handle) = conn_manager.spawn_abortable();

        Ok((sender, abort_handle))
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
                match self.start_new_connection_manager(socket_address).await {
                    Ok(res) => res,
                    Err(err) => {
                        warn!(
                            "failed to establish initial connection to {} - {}",
                            socket_address, err
                        );
                        return Err(err);
                    }
                };

            self.connections_managers
                .insert(socket_address, (new_manager_sender, abort_handle));
        }

        let manager = self.connections_managers.get_mut(&socket_address).unwrap();

        let framed_packet = FramedSphinxPacket::new(packet, packet_mode);

        let (res_tx, res_rx) = if wait_for_response {
            let (res_tx, res_rx) = oneshot::channel();
            (Some(res_tx), Some(res_rx))
        } else {
            (None, None)
        };

        if let Err(err) = manager.0.unbounded_send((framed_packet, res_tx)) {
            warn!(
                "Connection manager to {} has failed - {}",
                socket_address, err
            );
            self.connections_managers.remove(&socket_address);
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, err));
        }

        if let Some(res_rx) = res_rx {
            res_rx.await.unwrap()
        } else {
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
