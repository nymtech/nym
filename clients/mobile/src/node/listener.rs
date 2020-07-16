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

use crate::node::packet_processing::{MixProcessingResult, PacketProcessor};
use futures::channel::mpsc;
use log::*;
use nymsphinx::framing::SphinxCodec;
use nymsphinx::SphinxPacket;
use std::io;
use std::net::SocketAddr;
use tokio::runtime::Handle;
use tokio::stream::StreamExt;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;

async fn process_received_packet(
    sphinx_packet: SphinxPacket,
    packet_processor: PacketProcessor,
    forwarding_channel: mpsc::UnboundedSender<(SocketAddr, SphinxPacket)>,
) {
    // all processing incl. delay was done, the only thing left is to forward it
    match packet_processor.process_sphinx_packet(sphinx_packet).await {
        Err(e) => debug!("We failed to process received sphinx packet - {:?}", e),
        Ok(res) => match res {
            MixProcessingResult::ForwardHop(hop_address, forward_packet) => {
                // send our data to tcp client for forwarding. If forwarding fails, then it fails,
                // it's not like we can do anything about it
                //
                // in unbounded_send() failed it means that the receiver channel was disconnected
                // and hence something weird must have happened without a way of recovering
                forwarding_channel
                    .unbounded_send((hop_address, forward_packet))
                    .unwrap();
                packet_processor.report_sent(hop_address);
            }
            MixProcessingResult::LoopMessage => {
                warn!("Somehow processed a loop cover message that we haven't implemented yet!")
            }
        },
    }
}

async fn process_socket_connection(
    socket: tokio::net::TcpStream,
    packet_processor: PacketProcessor,
    forwarding_channel: mpsc::UnboundedSender<(SocketAddr, SphinxPacket)>,
) {
    let mut framed = Framed::new(socket, SphinxCodec);
    while let Some(sphinx_packet) = framed.next().await {
        match sphinx_packet {
            Ok(sphinx_packet) => {
                // we *really* need a worker pool here, because if we receive too many packets,
                // we will spawn too many tasks and starve CPU due to context switching.
                // (because presumably tokio has some concept of context switching in its
                // scheduler)
                tokio::spawn(process_received_packet(
                    sphinx_packet,
                    packet_processor.clone(),
                    forwarding_channel.clone(),
                ));
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
        framed.into_inner().peer_addr()
    );
}

pub(crate) fn run_socket_listener(
    handle: &Handle,
    addr: SocketAddr,
    packet_processor: PacketProcessor,
    forwarding_channel: mpsc::UnboundedSender<(SocketAddr, SphinxPacket)>,
) -> JoinHandle<io::Result<()>> {
    let handle_clone = handle.clone();
    handle.spawn(async move {
        let mut listener = tokio::net::TcpListener::bind(addr).await?;
        loop {
            let (socket, _) = listener.accept().await?;

            let thread_packet_processor = packet_processor.clone();
            let forwarding_channel_clone = forwarding_channel.clone();
            handle_clone.spawn(async move {
                process_socket_connection(
                    socket,
                    thread_packet_processor,
                    forwarding_channel_clone,
                )
                .await;
            });
        }
    })
}
