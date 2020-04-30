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

use crate::node::mixnet_handling::receiver::packet_processing::PacketProcessor;
use log::*;
use std::net::SocketAddr;
use tokio::{io::AsyncReadExt, prelude::*};

pub(crate) struct Handle<S: AsyncRead + AsyncWrite + Unpin> {
    peer_address: SocketAddr,
    socket_connection: S,
    packet_processor: PacketProcessor,
}

impl<S> Handle<S>
where
    S: AsyncRead + AsyncWrite + Unpin + 'static,
{
    // for time being we assume handle is always constructed from raw socket.
    // if we decide we want to change it, that's not too difficult
    pub(crate) fn new(
        peer_address: SocketAddr,
        conn: S,
        packet_processor: PacketProcessor,
    ) -> Self {
        Handle {
            peer_address,
            socket_connection: conn,
            packet_processor,
        }
    }

    async fn process_received_packet(
        packet_data: [u8; nymsphinx::PACKET_SIZE],
        mut packet_processor: PacketProcessor,
    ) {
        match packet_processor.process_sphinx_packet(packet_data).await {
            Ok(_) => trace!("successfully processed [and forwarded/stored] a final hop packet"),
            Err(e) => debug!("We failed to process received sphinx packet - {:?}", e),
        }
    }

    pub(crate) async fn start_handling(&mut self) {
        let mut buf = [0u8; nymsphinx::PACKET_SIZE];
        loop {
            match self.socket_connection.read_exact(&mut buf).await {
                // socket closed
                Ok(n) if n == 0 => {
                    trace!("Remote connection closed.");
                    return;
                }
                Ok(n) => {
                    // If I understand it correctly, this if should never be executed as if `read_exact`
                    // does not fill buffer, it will throw UnexpectedEof?
                    if n != nymsphinx::PACKET_SIZE {
                        error!("read data of different length than expected sphinx packet size - {} (expected {})", n, nymsphinx::PACKET_SIZE);
                        continue;
                    }

                    // we must be able to handle multiple packets from same connection independently
                    // TODO: but WE NEED to have some worker pool so that we do not spawn too many
                    // tasks
                    tokio::spawn(Self::process_received_packet(
                        buf,
                        self.packet_processor.clone(),
                    ))
                }
                Err(e) => {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        debug!("Read buffer was not fully filled. Most likely the client ({:?}) closed the connection.\
                   Also closing the connection on this end.", self.peer_address)
                    } else {
                        warn!(
                           "failed to read from socket (source: {:?}). Closing the connection; err = {:?}",
                           self.peer_address,
                           e
                       );
                    }
                    return;
                }
            };
        }
    }
}
