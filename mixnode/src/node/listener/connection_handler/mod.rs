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

use crate::node::listener::connection_handler::packet_processing::{
    MixProcessingResult, PacketProcessor,
};
use log::*;
use mixnet_client::forwarder::{ForwardedPacket, MixForwardingSender};
use nymsphinx::framing::codec::SphinxCodec;
use nymsphinx::framing::packet::FramedSphinxPacket;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio_util::codec::Framed;

pub(crate) mod packet_processing;

pub(crate) struct ConnectionHandler {
    packet_processor: PacketProcessor,
    forwarding_channel: MixForwardingSender,
}

impl ConnectionHandler {
    pub(crate) fn new(
        packet_processor: PacketProcessor,
        forwarding_channel: MixForwardingSender,
    ) -> Self {
        ConnectionHandler {
            packet_processor,
            forwarding_channel,
        }
    }

    pub(crate) fn clone_without_cache(&self) -> Self {
        ConnectionHandler {
            packet_processor: self.packet_processor.clone_without_cache(),
            forwarding_channel: self.forwarding_channel.clone(),
        }
    }

    fn forward_packet(&self, forward_packet: ForwardedPacket) {
        let routing_address = forward_packet.hop_adddress();
        // send our data to tcp client for forwarding. If forwarding fails, then it fails,
        // it's not like we can do anything about it
        //
        // in unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.forwarding_channel
            .unbounded_send(forward_packet)
            .unwrap();
        self.packet_processor.report_sent(routing_address);
    }

    async fn handle_received_packet(self: Arc<Self>, framed_sphinx_packet: FramedSphinxPacket) {
        //
        // TODO: here be replay attack detection - it will require similar key cache to the one in
        // packet processor for vpn packets,
        // question: can it also be per connection vs global?
        //

        // all processing including delaying, key caching, etc. was done, the only thing left is to forward it
        match self
            .packet_processor
            .process_received(framed_sphinx_packet)
            .await
        {
            Err(e) => debug!("We failed to process received sphinx packet - {:?}", e),
            Ok(res) => match res {
                MixProcessingResult::ForwardHop(forward_packet) => {
                    self.forward_packet(forward_packet)
                }
                MixProcessingResult::FinalHop(..) => {
                    warn!("Somehow processed a loop cover message that we haven't implemented yet!")
                }
            },
        }
    }

    pub(crate) async fn handle_connection(self, conn: TcpStream, remote: SocketAddr) {
        debug!("Starting connection handler for {:?}", remote);
        let this = Arc::new(self);
        let mut framed_conn = Framed::new(conn, SphinxCodec);
        while let Some(framed_sphinx_packet) = framed_conn.next().await {
            match framed_sphinx_packet {
                Ok(framed_sphinx_packet) => {
                    // TODO: benchmark spawning tokio task with full processing vs just processing it
                    // synchronously (without delaying inside of course,
                    // delay could be moved to a per-connection DelayQueue. The delay queue future
                    // could automatically just forward packet that is done being delayed)
                    // under higher load in single and multi-threaded situation.
                    //
                    // My gut feeling is saying that we might get some nice performance boost
                    // if we introduced the change
                    let this = Arc::clone(&this);
                    tokio::spawn(this.handle_received_packet(framed_sphinx_packet));
                }
                Err(err) => {
                    error!(
                        "The socket connection got corrupted with error: {:?}. Closing the socket",
                        err
                    );
                    return;
                }
            }
        }

        info!(
            "Closing connection from {:?}",
            framed_conn.into_inner().peer_addr()
        );
    }
}
