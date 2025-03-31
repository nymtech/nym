// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::shared::SharedData;
use futures::StreamExt;
use nym_metrics::nanos;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::codec::NymCodec;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_framing::processing::{
    finalize_packet_unwrapping, perform_partial_unwrapping, process_framed_packet,
    MixProcessingResult, MixProcessingResultData, PacketProcessingError, ProcessedFinalHop,
};
use nym_sphinx_types::Delay;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_util::codec::Framed;
use tracing::{debug, error, instrument, trace};

pub(crate) struct ConnectionHandler {
    shared: SharedData,
    mixnet_connection: Framed<TcpStream, NymCodec>,
    remote_address: SocketAddr,
}

impl Drop for ConnectionHandler {
    fn drop(&mut self) {
        self.shared
            .metrics
            .network
            .disconnected_ingress_mixnet_client()
    }
}

impl ConnectionHandler {
    pub(crate) fn new(
        shared: &SharedData,
        tcp_stream: TcpStream,
        remote_address: SocketAddr,
    ) -> Self {
        let shutdown = shared.shutdown.child_token(remote_address.to_string());
        shared.metrics.network.new_active_ingress_mixnet_client();

        ConnectionHandler {
            shared: SharedData {
                processing_config: shared.processing_config,
                sphinx_keys: shared.sphinx_keys.clone(),
                mixnet_forwarder: shared.mixnet_forwarder.clone(),
                final_hop: shared.final_hop.clone(),
                metrics: shared.metrics.clone(),
                shutdown,
            },
            remote_address,
            mixnet_connection: Framed::new(tcp_stream, NymCodec),
        }
    }

    /// Determine instant at which packet should get forwarded to the next hop.
    /// By using [`Instant`] rather than explicit [`Duration`] we minimise effects of
    /// the skew caused by being stuck in the channel queue.
    /// This method also clamps the maximum allowed delay so that nobody could send a bunch of packets
    /// with, for example, delays of 1 year thus causing denial of service
    fn create_delay_target(&self, delay: Option<Delay>) -> Option<Instant> {
        let delay = delay?.to_duration();
        let now = Instant::now();

        let delay = if delay > self.shared.processing_config.maximum_packet_delay {
            self.shared.processing_config.maximum_packet_delay
        } else {
            delay
        };
        trace!(
            "received packet will be delayed for {}ms",
            delay.as_millis()
        );

        Some(now + delay)
    }

    fn handle_forward_packet(&self, mix_packet: MixPacket, delay: Option<Delay>) {
        if !self.shared.processing_config.forward_hop_processing_enabled {
            trace!("this nym-node does not support forward hop packets");
            self.shared.dropped_forward_packet(self.remote_address.ip());
            return;
        }

        let forward_instant = self.create_delay_target(delay);
        self.shared.forward_mix_packet(mix_packet, forward_instant);
    }

    async fn handle_final_hop(&self, final_hop_data: ProcessedFinalHop) {
        if !self.shared.processing_config.final_hop_processing_enabled {
            trace!("this nym-node does not support final hop packets");
            self.shared
                .dropped_final_hop_packet(self.remote_address.ip());
            return;
        }

        let client = final_hop_data.destination;
        let message = final_hop_data.message;

        // if possible attempt to push message directly to the client
        match self.shared.try_push_message_to_client(client, message) {
            Err(unsent_plaintext) => {
                // if that failed, store it on disk (to be 🔥 soon...)
                match self
                    .shared
                    .store_processed_packet_payload(client, unsent_plaintext)
                    .await
                {
                    Err(err) => error!("Failed to store client data - {err}"),
                    Ok(_) => {
                        self.shared
                            .metrics
                            .mixnet
                            .egress
                            .add_disk_persisted_packet();
                        trace!("Stored packet for {client}")
                    }
                }
            }
            Ok(_) => trace!("Pushed received packet to {client}"),
        }

        // if we managed to either push message directly to the [online] client or store it at
        // its inbox, it means that it must exist at this gateway, hence we can send the
        // received ack back into the network
        self.shared.forward_ack_packet(final_hop_data.forward_ack);
    }

    async fn check_for_packet_replays(&self, replay_tag: &[u8]) -> bool {
        false
        // todo!()
    }

    async fn handle_received_packet_with_replay_detection(
        &self,
        packet: FramedNymPacket,
    ) -> Result<MixProcessingResult, PacketProcessingError> {
        // 1. derive and expand shared secret
        // also check the header integrity
        let partially_unwrapped = match perform_partial_unwrapping(
            packet.packet(),
            self.shared.sphinx_keys.private_key().as_ref(),
        ) {
            Ok(unwrapped) => unwrapped,
            Err(err) => {
                trace!("failed to process received mix packet: {err}");
                self.shared
                    .metrics
                    .mixnet
                    .ingress_malformed_packet(self.remote_address.ip());
                return Err(err);
            }
        };

        // 2. check for packet replay
        if let Some(replay_tag) = partially_unwrapped.replay_tag() {
            if self.check_for_packet_replays(replay_tag).await {
                self.shared
                    .metrics
                    .mixnet
                    .ingress_replayed_packet(self.remote_address.ip());
                return Err(PacketProcessingError::PacketReplay);
            }
        }

        // 3. process the rest of the packet
        finalize_packet_unwrapping(packet, partially_unwrapped)
    }

    #[instrument(skip(self, packet), level = "debug")]
    async fn handle_received_nym_packet(&self, packet: FramedNymPacket) {
        nanos!("handle_received_nym_packet", {
            // 1. attempt to unwrap the packet
            // if it's a sphinx packet attempt to do pre-processing and replay detection
            let unwrapped_packet = if packet.is_sphinx() {
                self.handle_received_packet_with_replay_detection(packet)
                    .await
            } else {
                process_framed_packet(packet, self.shared.sphinx_keys.private_key().as_ref())
            };

            // 2. increment our favourite metrics stats
            self.shared
                .update_metrics(&unwrapped_packet, self.remote_address.ip());

            // 3. forward the packet to the relevant sink (if enabled)
            match unwrapped_packet {
                Err(err) => trace!("failed to process received mix packet: {err}"),
                Ok(processed_packet) => match processed_packet.processing_data {
                    MixProcessingResultData::ForwardHop { packet, delay } => {
                        self.handle_forward_packet(packet, delay);
                    }
                    MixProcessingResultData::FinalHop { final_hop_data } => {
                        self.handle_final_hop(final_hop_data).await;
                    }
                },
            }
        });
    }

    #[instrument(
        skip(self),
        level = "debug",
        fields(
            remote = %self.remote_address
        )
    )]
    pub(crate) async fn handle_stream(&mut self) {
        loop {
            tokio::select! {
                biased;
                _ = self.shared.shutdown.cancelled() => {
                    trace!("connection handler: received shutdown");
                    break
                }
                maybe_framed_nym_packet = self.mixnet_connection.next() => {
                    match maybe_framed_nym_packet {
                        Some(Ok(packet)) => self.handle_received_nym_packet(packet).await,
                        Some(Err(err)) => {
                            debug!("connection got corrupted with: {err}");
                            return
                        }
                        None => {
                            debug!("connection got closed by the remote");
                            return
                        }
                    }
                }
            }
        }

        debug!("exiting and closing connection");
    }
}
