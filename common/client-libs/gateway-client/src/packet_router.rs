// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// JS: I personally don't like this name very much, but could not think of anything better.
// I will gladly take any suggestions on how to rename this.

use futures::channel::mpsc;
use log::*;
use nymsphinx::addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
use nymsphinx::params::packet_sizes::PacketSize;
#[cfg(not(target_arch = "wasm32"))]
use task::ShutdownListener;

use crate::error::GatewayClientError;

pub type MixnetMessageSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type MixnetMessageReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

pub type AcknowledgementSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub type AcknowledgementReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

#[derive(Clone, Debug)]
pub struct PacketRouter {
    ack_sender: AcknowledgementSender,
    mixnet_message_sender: MixnetMessageSender,
    #[cfg(not(target_arch = "wasm32"))]
    shutdown: Option<ShutdownListener>,
}

impl PacketRouter {
    pub fn new(
        ack_sender: AcknowledgementSender,
        mixnet_message_sender: MixnetMessageSender,
        #[cfg(not(target_arch = "wasm32"))] shutdown: Option<ShutdownListener>,
    ) -> Self {
        PacketRouter {
            ack_sender,
            mixnet_message_sender,
            #[cfg(not(target_arch = "wasm32"))]
            shutdown,
        }
    }

    pub fn route_received(
        &mut self,
        unwrapped_packets: Vec<Vec<u8>>,
    ) -> Result<(), GatewayClientError> {
        let mut received_messages = Vec::new();
        let mut received_acks = Vec::new();

        // remember: gateway removes final layer of sphinx encryption and from the unwrapped
        // data he takes the SURB-ACK and first hop address.
        // currently SURB-ACKs are attached in EVERY packet, even cover, so this is always true
        let ack_overhead = PacketSize::AckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN;

        for received_packet in unwrapped_packets {
            if received_packet.len() == PacketSize::AckPacket.plaintext_size() {
                received_acks.push(received_packet);
            } else if received_packet.len()
                == PacketSize::RegularPacket.plaintext_size() - ack_overhead
            {
                trace!("routing regular packet");
                received_messages.push(received_packet);
            } else if received_packet.len()
                == PacketSize::ExtendedPacket8.plaintext_size() - ack_overhead
            {
                trace!("routing extended8 packet");
                received_messages.push(received_packet);
            } else if received_packet.len()
                == PacketSize::ExtendedPacket16.plaintext_size() - ack_overhead
            {
                trace!("routing extended16 packet");
                received_messages.push(received_packet);
            } else if received_packet.len()
                == PacketSize::ExtendedPacket32.plaintext_size() - ack_overhead
            {
                trace!("routing extended32 packet");
                received_messages.push(received_packet);
            } else {
                // this can happen if other clients are not padding their messages
                warn!("Received message of unexpected size. Probably from an outdated client... len: {}", received_packet.len());
                received_messages.push(received_packet);
            }
        }

        if !received_messages.is_empty() {
            trace!("routing 'real'");
            if let Err(err) = self.mixnet_message_sender.unbounded_send(received_messages) {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(shutdown) = &mut self.shutdown {
                    if shutdown.is_shutdown_poll() {
                        // This should ideally not happen, but it's ok
                        log::warn!("Failed to send mixnet message due to receiver task shutdown");
                        return Err(GatewayClientError::MixnetMsgSenderFailedToSend);
                    }
                }
                // This should never happen during ordinary operation the way it's currently used.
                // Abort to be on the safe side
                panic!("Failed to send mixnet message: {:?}", err);
            }
        }

        if !received_acks.is_empty() {
            trace!("routing acks");
            if let Err(e) = self.ack_sender.unbounded_send(received_acks) {
                error!("failed to send ack: {:?}", e);
            };
        }
        Ok(())
    }
}
