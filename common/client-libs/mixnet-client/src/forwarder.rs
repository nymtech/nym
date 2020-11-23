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

use crate::client::{Client, Config};
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
