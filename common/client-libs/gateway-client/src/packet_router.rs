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

// JS: I personally don't like this name very much, but could not think of anything better.
// I will gladly take any suggestions on how to rename this.

use futures::channel::mpsc;
use log::*;
use nymsphinx::addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
use nymsphinx::params::packet_sizes::PacketSize;

pub type MixnetMessageSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type MixnetMessageReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

pub type AcknowledgementSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type AcknowledgementReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

#[derive(Clone, Debug)]
pub struct PacketRouter {
    ack_sender: AcknowledgementSender,
    mixnet_message_sender: MixnetMessageSender,
}

impl PacketRouter {
    pub fn new(
        ack_sender: AcknowledgementSender,
        mixnet_message_sender: MixnetMessageSender,
    ) -> Self {
        PacketRouter {
            ack_sender,
            mixnet_message_sender,
        }
    }

    pub fn route_received(&self, unwrapped_packets: Vec<Vec<u8>>) {
        let mut received_messages = Vec::new();
        let mut received_acks = Vec::new();

        // remember: gateway removes final layer of sphinx encryption and from the unwrapped
        // data he takes the SURB-ACK and first hop address.
        // currently SURB-ACKs are attached in EVERY packet, even cover, so this is always true
        let ack_overhead = PacketSize::ACKPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN;

        for received_packet in unwrapped_packets {
            if received_packet.len() == PacketSize::ACKPacket.plaintext_size() {
                received_acks.push(received_packet);
            } else if received_packet.len()
                == PacketSize::RegularPacket.plaintext_size() - ack_overhead
            {
                received_messages.push(received_packet);
            } else if received_packet.len()
                == PacketSize::ExtendedPacket.plaintext_size() - ack_overhead
            {
                warn!("received extended packet? Did not expect this...");
                received_messages.push(received_packet);
            } else {
                // this can happen if other clients are not padding their messages
                warn!("Received message of unexpected size. Probably from an outdated client... len: {}", received_packet.len());
                received_messages.push(received_packet);
            }
        }

        // due to how we are currently using it, those unwraps can't fail, but if we ever
        // wanted to make `gateway-client` into some more generic library, we would probably need
        // to catch that error or something.
        if !received_messages.is_empty() {
            trace!("routing 'real'");
            self.mixnet_message_sender
                .unbounded_send(received_messages)
                .unwrap();
        }

        if !received_acks.is_empty() {
            trace!("routing acks");
            self.ack_sender.unbounded_send(received_acks).unwrap();
        }
    }
}
