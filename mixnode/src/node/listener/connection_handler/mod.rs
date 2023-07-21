// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::identity::{PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use crate::node::listener::connection_handler::packet_processing::{
    MixProcessingResult, PacketProcessor,
};
use crate::node::packet_delayforwarder::PacketDelayForwardSender;
use crate::node::TaskClient;
use futures::StreamExt;
use nym_client_core::client::topology_control::accessor::TopologyAccessor;
use nym_crypto::asymmetric::identity;
use nym_mixnode_common::measure;
use nym_noise::upgrade_noise_responder;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::framing::codec::NymCodec;
use nym_sphinx::framing::packet::FramedNymPacket;
use nym_sphinx::Delay as SphinxDelay;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;
#[cfg(feature = "cpucycles")]
use tracing::{error, info, instrument};

pub(crate) mod packet_processing;

#[derive(Clone)]
pub(crate) struct ConnectionHandler {
    packet_processor: PacketProcessor,
    delay_forwarding_channel: PacketDelayForwardSender,
    topology_access: TopologyAccessor,
    private_identity_key: [u8; SECRET_KEY_LENGTH],
    public_identity_key: [u8; PUBLIC_KEY_LENGTH],
}

impl ConnectionHandler {
    pub(crate) fn new(
        packet_processor: PacketProcessor,
        delay_forwarding_channel: PacketDelayForwardSender,
        topology_access: TopologyAccessor,
        identity_key: &identity::KeyPair,
    ) -> Self {
        ConnectionHandler {
            packet_processor,
            delay_forwarding_channel,
            topology_access,
            public_identity_key: identity_key.public_key().to_bytes(),
            private_identity_key: identity_key.private_key().to_bytes(),
        }
    }

    fn delay_and_forward_packet(&self, mix_packet: MixPacket, delay: Option<SphinxDelay>) {
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

        let topology_permit = self.topology_access.get_read_permit().await;
        let topology_ref = match topology_permit.try_get_raw_topology_ref() {
            Ok(topology) => topology,
            Err(err) => {
                error!("Cannot connect to {remote}, due to topology error - {err}");
                return;
            }
        };

        let noise_stream = match upgrade_noise_responder(
            conn,
            topology_ref,
            &self.private_identity_key,
            &self.public_identity_key,
        ) {
            Ok(noise_stream) => noise_stream,
            Err(err) => {
                error!("Failed to perform Noise handshake with {remote} - {err}");
                return;
            }
        };
        let mut framed_conn = Framed::new(noise_stream, NymCodec);
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

        info!("Closing connection from {:?}", remote);
        log::trace!("ConnectionHandler: Exiting");
    }
}
