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
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use nymsphinx::params::PacketMode;
use nymsphinx::SphinxPacket;
use std::time::Duration;

pub type MixForwardingSender = mpsc::UnboundedSender<ForwardedPacket>;
type MixForwardingReceiver = mpsc::UnboundedReceiver<ForwardedPacket>;

pub struct ForwardedPacket {
    hop_address: NymNodeRoutingAddress,
    packet: SphinxPacket,
    packet_mode: PacketMode,
}

impl ForwardedPacket {
    pub fn new(
        hop_address: NymNodeRoutingAddress,
        packet: SphinxPacket,
        packet_mode: PacketMode,
    ) -> Self {
        ForwardedPacket {
            hop_address,
            packet,
            packet_mode,
        }
    }

    pub fn hop_adddress(&self) -> NymNodeRoutingAddress {
        self.hop_address
    }

    pub fn packet(&self) -> &SphinxPacket {
        &self.packet
    }

    pub fn packet_mode(&self) -> PacketMode {
        self.packet_mode
    }
}

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
    ) -> (PacketForwarder, MixForwardingSender) {
        let client_config = Config::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
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
        while let Some(forwarded_packet) = self.packet_receiver.next().await {
            trace!(
                "Going to forward packet to {:?}",
                forwarded_packet.hop_address
            );
            // we don't care about responses, we just want to fire packets
            // as quickly as possible
            self.mixnet_client
                .send(
                    forwarded_packet.hop_address,
                    forwarded_packet.packet,
                    forwarded_packet.packet_mode,
                    false,
                )
                .await
                .unwrap(); // if we're not waiting for response, we MUST get an Ok
        }
    }
}
