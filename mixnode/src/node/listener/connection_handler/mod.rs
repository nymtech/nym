// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::listener::connection_handler::packet_processing::{
    MixProcessingResult, PacketProcessor,
};
use crate::node::packet_delayforwarder::PacketDelayForwardSender;
use crate::node::TaskClient;
use futures::StreamExt;
use log::{debug, error, info, warn};
use nym_mixnode_common::forward_travel::AllowedEgress;
use nym_mixnode_common::measure;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::framing::codec::NymCodec;
use nym_sphinx::framing::packet::FramedNymPacket;
use nym_sphinx::Delay as SphinxDelay;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;

#[cfg(feature = "cpucycles")]
use tracing::instrument;

pub(crate) mod packet_processing;

#[derive(Clone)]
pub(crate) struct ConnectionHandler {
    allowed_egress: AllowedEgress,
    packet_processor: PacketProcessor,
    delay_forwarding_channel: PacketDelayForwardSender,
}

impl ConnectionHandler {
    pub(crate) fn new(
        allowed_egress: AllowedEgress,
        packet_processor: PacketProcessor,
        delay_forwarding_channel: PacketDelayForwardSender,
    ) -> Self {
        ConnectionHandler {
            allowed_egress,
            packet_processor,
            delay_forwarding_channel,
        }
    }

    fn delay_and_forward_packet(&self, mix_packet: MixPacket, delay: Option<SphinxDelay>) {
        let next_hop: SocketAddr = mix_packet.next_hop().into();

        // TODO: another option is to move this filter
        // (which is used by EVERY `ConnectionHandler`, so potentially hundreds of times)
        // to the mixnet client where we could be filtering at the time of attempting to open new outbound connections
        // However, in that case we'd have gone through the troubles of possibly unnecessarily delaying the packet
        if !self.allowed_egress.is_allowed(next_hop.ip()) {
            // TODO: perhaps this should get lowered in severity?
            warn!("received an packet that was meant to get forwarded to {next_hop}, but this address does not belong to any node on the next layer - dropping the packet");
            return;
        }

        // determine instant at which packet should get forwarded. this way we minimise effect of
        // being stuck in the queue [of the channel] to get inserted into the delay queue
        let forward_instant = delay.map(|delay| Instant::now() + delay.to_duration());

        // if unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.delay_forwarding_channel
            .unbounded_send((mix_packet, forward_instant))
            .expect("the delay-forwarder has died!");
    }

    #[cfg_attr(
        feature = "cpucycles",
        instrument(skip(self, framed_sphinx_packet), fields(cpucycles))
    )]
    fn handle_received_packet(&self, framed_sphinx_packet: FramedNymPacket) {
        //
        // TODO: here be replay attack detection - it will require similar key cache to the one in
        // packet processor for vpn packets,
        // question: can it also be per connection vs global?
        //

        // all processing such, key caching, etc. was done.
        // however, if it was a forward hop, we still need to delay it
        measure!({
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
        })
    }

    pub(crate) async fn handle_connection(
        self,
        conn: TcpStream,
        remote: SocketAddr,
        mut shutdown: TaskClient,
    ) {
        debug!("Starting connection handler for {:?}", remote);
        shutdown.mark_as_success();
        let mut framed_conn = Framed::new(conn, NymCodec);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("ConnectionHandler: received shutdown");
                }
                framed_sphinx_packet = framed_conn.next() => {
                    match framed_sphinx_packet {
                        Some(Ok(framed_sphinx_packet)) => {
                            // TODO: benchmark spawning tokio task with full processing vs just processing it
                            // synchronously (without delaying inside of course,
                            // delay is moved to a global DelayQueue)
                            // under higher load in single and multi-threaded situation.

                            // in theory we could process multiple sphinx packet from the same connection in parallel,
                            // but we already handle multiple concurrent connections so if anything, making
                            // that change would only slow things down
                            self.handle_received_packet(framed_sphinx_packet);
                        }
                        Some(Err(err)) => {
                            error!(
                                "{remote:?} - The socket connection got corrupted with error: {err}. Closing the socket",
                            );
                            return;
                        }
                        None => break, // stream got closed by remote
                    }
                },
            }
        }

        info!(
            "Closing connection from {:?}",
            framed_conn.into_inner().peer_addr()
        );
        log::trace!("ConnectionHandler: Exiting");
    }
}
