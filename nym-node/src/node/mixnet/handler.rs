// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::shared::SharedData;
use futures::StreamExt;
use nym_metrics::nanos;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::codec::NymCodec;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_framing::processing::{
    process_framed_packet, MixProcessingResult, ProcessedFinalHop,
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
        let mut task_client = shared.task_client.fork(remote_address.to_string());
        // we don't want dropped connections to cause global shutdown
        task_client.disarm();

        shared.metrics.network.new_active_ingress_mixnet_client();

        ConnectionHandler {
            shared: SharedData {
                processing_config: shared.processing_config.clone(),
                sphinx_key: shared.sphinx_key.clone(),
                mixnet_forwarder: shared.mixnet_forwarder.clone(),
                final_hop: shared.final_hop.clone(),
                metrics: shared.metrics.clone(),
                task_client,
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
                    Ok(_) => trace!("Stored packet for {client}"),
                }
            }
            Ok(_) => trace!("Pushed received packet to {client}"),
        }

        // if we managed to either push message directly to the [online] client or store it at
        // its inbox, it means that it must exist at this gateway, hence we can send the
        // received ack back into the network
        self.shared.forward_ack_packet(final_hop_data.forward_ack);
    }

    #[instrument(skip(self, packet), level = "debug")]
    async fn handle_received_nym_packet(&self, packet: FramedNymPacket) {
        // TODO: here be replay attack detection with bloomfilters and all the fancy stuff
        //

        nanos!("handle_received_nym_packet", {
            // 1. attempt to unwrap the packet
            let unwrapped_packet = process_framed_packet(packet, &self.shared.sphinx_key);

            // 2. increment our favourite metrics stats
            self.shared
                .update_metrics(&unwrapped_packet, self.remote_address.ip());

            // 3. forward the packet to the relevant sink (if enabled)
            match unwrapped_packet {
                Err(err) => trace!("failed to process received mix packet: {err}"),
                Ok(MixProcessingResult::ForwardHop(forward_packet, delay)) => {
                    self.handle_forward_packet(forward_packet, delay);
                }
                Ok(MixProcessingResult::FinalHop(final_hop_data)) => {
                    self.handle_final_hop(final_hop_data).await;
                }
            }
        })
    }

    #[instrument(
        skip(self),
        level = "debug",
        fields(
            remote = %self.remote_address
        )
    )]
    pub(crate) async fn handle_stream(&mut self) {
        while !self.shared.task_client.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shared.task_client.recv() => {
                    trace!("connection handler: received shutdown");
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
