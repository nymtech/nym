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
use std::io;
use std::net::SocketAddr;
use tokio::prelude::*;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

async fn process_received_packet(
    packet_data: [u8; nymsphinx::PACKET_SIZE],
    packet_processor: PacketProcessor,
    forwarding_channel: mpsc::UnboundedSender<(SocketAddr, Vec<u8>)>,
) {
    // all processing incl. delay was done, the only thing left is to forward it
    match packet_processor.process_sphinx_packet(packet_data).await {
        Err(e) => debug!("We failed to process received sphinx packet - {:?}", e),
        Ok(res) => match res {
            MixProcessingResult::ForwardHop(hop_address, hop_data) => {
                // send our data to tcp client for forwarding. If forwarding fails, then it fails,
                // it's not like we can do anything about it
                //
                // in unbounded_send() failed it means that the receiver channel was disconnected
                // and hence something weird must have happened without a way of recovering
                forwarding_channel
                    .unbounded_send((hop_address, hop_data))
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
    mut socket: tokio::net::TcpStream,
    packet_processor: PacketProcessor,
    forwarding_channel: mpsc::UnboundedSender<(SocketAddr, Vec<u8>)>,
) {
    let mut buf = [0u8; nymsphinx::PACKET_SIZE];
    loop {
        match socket.read_exact(&mut buf).await {
            // socket closed
            Ok(n) if n == 0 => {
                trace!("Remote connection closed.");
                return;
            }
            Ok(n) => {
                // If I understand it correctly, this if should never be executed as if `read_exact`
                // does not fill buffer, it will throw UnexpectedEof?
                if n != nymsphinx::PACKET_SIZE {
                    warn!("read data of different length than expected sphinx packet size - {} (expected {})", n, nymsphinx::PACKET_SIZE);
                    continue;
                }
                // we must be able to handle multiple packets from same connection independently
                tokio::spawn(process_received_packet(
                    buf,
                    // note: processing_data is relatively cheap (and safe) to clone -
                    // it contains arc to private key and metrics reporter (which is just
                    // a single mpsc unbounded sender)
                    packet_processor.clone(),
                    forwarding_channel.clone(),
                ))
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    debug!("Read buffer was not fully filled. Most likely the client ({:?}) closed the connection.\
                    Also closing the connection on this end.", socket.peer_addr())
                } else {
                    warn!(
                        "failed to read from socket (source: {:?}). Closing the connection; err = {:?}",
                        socket.peer_addr(),
                        e
                    );
                }
                return;
            }
        };
    }
}

pub(crate) fn run_socket_listener(
    handle: &Handle,
    addr: SocketAddr,
    packet_processor: PacketProcessor,
    forwarding_channel: mpsc::UnboundedSender<(SocketAddr, Vec<u8>)>,
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
