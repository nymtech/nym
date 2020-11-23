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
use futures::channel::mpsc::Receiver;
use futures::channel::{mpsc, oneshot};
use futures::future::AbortHandle;
use futures::SinkExt;
use log::*;
use nymsphinx::framing::codec::SphinxCodec;
use nymsphinx::framing::packet::FramedSphinxPacket;
use nymsphinx::params::PacketMode;
use nymsphinx::{addressing::nodes::NymNodeRoutingAddress, SphinxPacket};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio::time::Instant;
use tokio_util::codec::Framed;

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

// connection to {} seems to not be able to handle all the traffic - dropping the current packet
const MAX_CONN_BUF: usize = 16;

pub struct Client {
    connections_managers: HashMap<SocketAddr, (ConnectionManagerSender, AbortHandle)>,

    conn_new: HashMap<NymNodeRoutingAddress, mpsc::Sender<FramedSphinxPacket>>,

    maximum_reconnection_backoff: Duration,
    initial_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
    maximum_reconnection_attempts: u32,
}

// pub(crate) struct ConnectionManager2 {
//     receiver: Receiver<SphinxPacket>,
// }
//
// impl ConnectionManager2 {
//     pub(crate) fn new(address: SocketAddr) -> Result<Self, io::Error>{
//         let connection_timeout = Duration::from_secs(1);
//
//         let conn = match std::net::TcpStream::connect_timeout(&address, connection_timeout) {
//             Ok(stream) => {
//                 let tokio_stream = tokio::net::TcpStream::from_std(stream).unwrap();
//                 debug!("managed to establish initial connection to {}", address);
//                 ConnectionState::Writing(ConnectionWriter::new(tokio_stream))
//             }
//             Err(err) => {
//                 warn!("failed to connect to {} within {}", address, connection_timeout);
//                 return Err(err)
//             },
//         };
//
//
//
//
//
//     }
// }

//
// struct ConnSender {
//     sender: mpsc::Sender<FramedSphinxPacket>,
//     is_accepting: Arc<AtomicBool>,
// }

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            conn_new: HashMap::new(),
            connections_managers: HashMap::new(),
            initial_reconnection_backoff: config.initial_reconnection_backoff,
            maximum_reconnection_backoff: config.maximum_reconnection_backoff,
            initial_connection_timeout: config.initial_connection_timeout,
            maximum_reconnection_attempts: config.maximum_reconnection_attempts,
        }
    }

    async fn manage_connection(
        address: SocketAddr,
        mut receiver: mpsc::Receiver<FramedSphinxPacket>,
        connection_timeout: Duration,
    ) -> io::Result<()> {
        let mut conn = match std::net::TcpStream::connect_timeout(&address, connection_timeout) {
            Ok(stream) => {
                let tokio_stream = tokio::net::TcpStream::from_std(stream).unwrap();
                debug!("managed to establish initial connection to {}", address);
                Framed::new(tokio_stream, SphinxCodec)
            }
            Err(err) => {
                warn!(
                    "failed to connect to {} within {:?}",
                    address, connection_timeout
                );
                return Err(err);
            }
        };

        while let Some(packet) = receiver.next().await {
            if let Err(err) = conn.send(packet).await {
                warn!("Failed to forward packet to {} - {:?}", address, err);
                // there's no point in draining the channel, it's incredibly unlikely further
                // messages might succeed
                break;
            } else {
                trace!("managed to forward packet to {}", address)
            }
        }

        // if we got here it means the mixnet client was dropped
        debug!(
            "connection manager to {} is finished. Presumably mixnet client got dropped",
            address
        );
        Ok(())
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

    fn make_connection(&mut self, address: NymNodeRoutingAddress) {
        let (sender, receiver) = mpsc::channel(MAX_CONN_BUF);
        tokio::spawn(Self::manage_connection(
            address.clone().into(),
            receiver,
            self.initial_connection_timeout,
        ));

        self.conn_new.insert(address, sender);
    }

    pub fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: SphinxPacket,
        packet_mode: PacketMode,
    ) {
        trace!("Sending packet to {:?}", address);

        if let Some(sender) = self.conn_new.get_mut(&address) {
            let framed_packet = FramedSphinxPacket::new(packet, packet_mode);
            if let Err(err) = sender.try_send(framed_packet) {
                if err.is_full() {
                    // TODO: perhaps change this to `debug` instead?
                    warn!("connection to {} seems to not be able to handle all the traffic - dropping the current packet", address);
                } else if err.is_disconnected() {
                    // TODO ONLY if not in reconnection backoff
                    // TODO ONLY if not in reconnection backoff

                    // the connection is dead - make a new one instead
                    warn!(
                        "connection to {} seems to have died. remaking the connection",
                        address
                    );
                    self.make_connection(address)
                }
            }
        } else {
            // there was never a connection to begin with
            info!("establishing initial connection to {}", address);
            self.make_connection(address)
        }

        //
        // let socket_address = address.into();
        //
        // if !self.connections_managers.contains_key(&socket_address) {
        //     debug!(
        //         "There is no existing connection to {:?} - it will be established now",
        //         address
        //     );
        //
        //     let (sender, receiver) = mpsc::unbounded();
        //
        //     let (new_manager_sender, abort_handle) =
        //         match self.start_new_connection_manager(socket_address).await {
        //             Ok(res) => res,
        //             Err(err) => {
        //                 debug!(
        //                     "failed to establish initial connection to {} - {}",
        //                     socket_address, err
        //                 );
        //                 return Err(err);
        //             }
        //         };
        //
        //     self.connections_managers
        //         .insert(socket_address, (new_manager_sender, abort_handle));
        // }
        //
        // let manager = self.connections_managers.get_mut(&socket_address).unwrap();
        //
        // let framed_packet = FramedSphinxPacket::new(packet, packet_mode);
        //
        // if let Err(err) = manager.0.unbounded_send((framed_packet, None)) {
        //     warn!(
        //         "Connection manager to {} has failed - {}",
        //         socket_address, err
        //     );
        //     self.connections_managers.remove(&socket_address);
        //     Err(io::Error::new(io::ErrorKind::BrokenPipe, err))
        // } else {
        //     Ok(())
        // }
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
        self.send_without_response(address, packet, packet_mode);

        Ok(())

        // todo!()
        // trace!("Sending packet to {:?}", address);
        // let socket_address = address.into();
        // info!("sending to {} start!", address);
        // if !self.connections_managers.contains_key(&socket_address) {
        //     debug!(
        //         "There is no existing connection to {:?} - it will be established now",
        //         address
        //     );
        //
        //     let (new_manager_sender, abort_handle) =
        //         match self.start_new_connection_manager(socket_address).await {
        //             Ok(res) => res,
        //             Err(err) => {
        //                 debug!(
        //                     "failed to establish initial connection to {} - {}",
        //                     socket_address, err
        //                 );
        //                 return Err(err);
        //             }
        //         };
        //
        //     self.connections_managers
        //         .insert(socket_address, (new_manager_sender, abort_handle));
        // }
        //
        // let manager = self.connections_managers.get_mut(&socket_address).unwrap();
        //
        // let framed_packet = FramedSphinxPacket::new(packet, packet_mode);
        //
        // let (res_tx, res_rx) = if wait_for_response {
        //     let (res_tx, res_rx) = oneshot::channel();
        //     (Some(res_tx), Some(res_rx))
        // } else {
        //     (None, None)
        // };
        //
        // if let Err(err) = manager.0.unbounded_send((framed_packet, res_tx)) {
        //     warn!(
        //         "Connection manager to {} has failed - {}",
        //         socket_address, err
        //     );
        //     self.connections_managers.remove(&socket_address);
        //     return Err(io::Error::new(io::ErrorKind::BrokenPipe, err));
        // }
        //
        // info!(
        //     "sending to {} done (kinda, it shouldnt be blocked at very least!)!",
        //     address
        // );
        //
        // if let Some(res_rx) = res_rx {
        //     res_rx.await.unwrap()
        // } else {
        //     Ok(())
        // }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        for (_, abort_handle) in self.connections_managers.values() {
            abort_handle.abort()
        }
    }
}
