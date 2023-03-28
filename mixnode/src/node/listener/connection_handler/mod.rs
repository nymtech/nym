// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::listener::connection_handler::packet_processing::{
    MixProcessingResult, PacketProcessor,
};
use crate::node::packet_delayforwarder::PacketDelayForwardSender;
use crate::node::TaskClient;
use futures::StreamExt;
use tracing::{error, info, trace, debug, warn};
use tracing::*;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketSize;
use nym_sphinx::framing::codec::SphinxCodec;
use nym_sphinx::framing::packet::FramedSphinxPacket;
use nym_sphinx::Delay as SphinxDelay;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;
use std::time::{SystemTime,UNIX_EPOCH};

pub(crate) mod packet_processing;

#[derive(Clone)]
pub(crate) struct ConnectionHandler {
    packet_processor: PacketProcessor,
    delay_forwarding_channel: PacketDelayForwardSender,
}

impl ConnectionHandler {
    pub(crate) fn new(
        packet_processor: PacketProcessor,
        delay_forwarding_channel: PacketDelayForwardSender,
    ) -> Self {
        ConnectionHandler {
            packet_processor,
            delay_forwarding_channel,
        }
    }
    #[instrument(level="debug", skip_all, "Sending to packet forwarder", fields(packet_size))]
    fn delay_and_forward_packet(&self, mix_packet: MixPacket, delay: Option<SphinxDelay>) {
        // determine instant at which packet should get forwarded. this way we minimise effect of
        // being stuck in the queue [of the channel] to get inserted into the delay queue
        let packet_size = PacketSize::get_type(mix_packet.sphinx_packet().len()).unwrap();
        Span::current().record("packet_size", field::debug(packet_size));
        let forward_instant = delay.map(|delay| Instant::now() + delay.to_duration());

        // if unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.delay_forwarding_channel
            .unbounded_send((mix_packet, forward_instant))
            .expect("the delay-forwarder has died!");
    }
    #[instrument(level="info", skip_all, "Handling packet",fields(packet_size=?framed_sphinx_packet.packet_size()))]
    fn handle_received_packet(&self, framed_sphinx_packet: FramedSphinxPacket) {
        //
        // TODO: here be replay attack detection - it will require similar key cache to the one in
        // packet processor for vpn packets,
        // question: can it also be per connection vs global?
        //

        // all processing such, key caching, etc. was done.
        // however, if it was a forward hop, we still need to delay it
        match self.packet_processor.process_received(framed_sphinx_packet) {
            Err(err) => debug!("We failed to process received sphinx packet - {err}"),
            Ok(res) => match res {
                MixProcessingResult::ForwardHop(forward_packet, delay) => {
                    self.delay_and_forward_packet(forward_packet, delay)
                }
                MixProcessingResult::FinalHop(..) => {
                    warn!("Somehow processed a loop cover message that we haven't implemented yet!")
                }
            },
        }
    }
    #[instrument(level="info", skip_all, "Connection handling", fields(address=%remote))]
    pub(crate) async fn handle_connection(
        self,
        conn: TcpStream,
        remote: SocketAddr,
        mut shutdown: TaskClient,
    ) {
        debug!("Starting connection handler for {:?}", remote);
        shutdown.mark_as_success();
        let mut framed_conn = Framed::new(conn, SphinxCodec);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    trace!("ConnectionHandler: received shutdown");
                }
                Some(framed_sphinx_packet) = framed_conn.next() => {
                    match framed_sphinx_packet {
                        Ok(framed_sphinx_packet) => {
                            // TODO: benchmark spawning tokio task with full processing vs just processing it
                            // synchronously (without delaying inside of course,
                            // delay is moved to a global DelayQueue)
                            // under higher load in single and multi-threaded situation.

                            // in theory we could process multiple sphinx packet from the same connection in parallel,
                            // but we already handle multiple concurrent connections so if anything, making
                            // that change would only slow things down
                            //println!("{:?}_In_{:?}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(), framed_sphinx_packet.packet_size().size());
                            self.handle_received_packet(framed_sphinx_packet);
                            //println!("{:?}_Processed_{:?}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros(), mix_packet.sphinx_packet().len());
                        }
                        Err(err) => {
                            error!(
                                "The socket connection got corrupted with error: {err}. Closing the socket",
                            );
                            return;
                        }
                    }
                },
            }
        }

        info!(
            "Closing connection from {:?}",
            framed_conn.into_inner().peer_addr()
        );
        trace!("ConnectionHandler: Exiting");
    }
}
