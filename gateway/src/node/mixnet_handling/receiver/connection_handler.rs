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
use nymsphinx::framing::SphinxCodec;
use nymsphinx::SphinxPacket;
use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::stream::StreamExt;
use tokio_util::codec::Framed;

pub(crate) struct Handle<S: AsyncRead + AsyncWrite + Unpin> {
    peer_address: SocketAddr,
    framed_connection: Framed<S, SphinxCodec>,
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
        // we expect only to receive sphinx packets on this socket, so let's frame it here
        let framed = Framed::new(conn, SphinxCodec);
        Handle {
            peer_address,
            framed_connection: framed,
            packet_processor,
        }
    }

    async fn process_received_packet(
        sphinx_packet: SphinxPacket,
        mut packet_processor: PacketProcessor,
    ) {
        match packet_processor.process_sphinx_packet(sphinx_packet).await {
            Ok(_) => trace!("successfully processed [and forwarded/stored] a final hop packet"),
            Err(e) => debug!("We failed to process received sphinx packet - {:?}", e),
        }
    }

    pub(crate) async fn start_handling(&mut self) {
        while let Some(sphinx_packet) = self.framed_connection.next().await {
            match sphinx_packet {
                Ok(sphinx_packet) => {
                    // we *really* need a worker pool here, because if we receive too many packets,
                    // we will spawn too many tasks and starve CPU due to context switching.
                    // (because presumably tokio has some concept of context switching in its
                    // scheduler)
                    tokio::spawn(Self::process_received_packet(
                        sphinx_packet,
                        self.packet_processor.clone(),
                    ));
                }
                Err(err) => {
                    error!("The socket connection got corrupted with error: {:?}. Closing the socket", err)
                    return
                },
            }
        }
        info!("Closing connection from {:?}", self.peer_address);
    }
}
