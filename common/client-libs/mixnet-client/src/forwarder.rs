// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::{Client, Config, SendWithoutResponse};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nymsphinx::forwarding::packet::MixPacket;
use std::time::Duration;

pub type MixForwardingSender = mpsc::UnboundedSender<MixPacket>;
type MixForwardingReceiver = mpsc::UnboundedReceiver<MixPacket>;

/// A specialisation of client such that it forwards any received packets on the channel into the
/// mix network immediately, i.e. will not try to listen for any responses.
pub struct PacketForwarder {
    mixnet_client: Client,
    packet_receiver: MixForwardingReceiver,
}

impl PacketForwarder {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_connection_buffer_size: usize,
    ) -> (PacketForwarder, MixForwardingSender) {
        let client_config = Config::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_connection_buffer_size,
        );

        let (packet_sender, packet_receiver) = mpsc::unbounded();

        (
            PacketForwarder {
                mixnet_client: Client::new(client_config),
                packet_receiver,
            },
            packet_sender,
        )
    }

    pub async fn run(&mut self) {
        while let Some(mix_packet) = self.packet_receiver.next().await {
            trace!("Going to forward packet to {:?}", mix_packet.next_hop());

            let next_hop = mix_packet.next_hop();
            let packet_mode = mix_packet.packet_mode();
            let sphinx_packet = mix_packet.into_sphinx_packet();
            // we don't care about responses, we just want to fire packets
            // as quickly as possible

            if let Err(err) =
                self.mixnet_client
                    .send_without_response(next_hop, sphinx_packet, packet_mode)
            {
                debug!("failed to forward the packet - {}", err)
            }
        }
    }
}
