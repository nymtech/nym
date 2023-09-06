// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{error, trace, warn};
use nym_crypto::asymmetric::identity;
use nym_gateway_client::error::GatewayClientError;
use nym_gateway_client::GatewayClient;
use nym_sphinx::addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketSize;
use std::fmt::{Debug, Formatter};

// #[derive(Debug)]
// pub struct TempInnerError;

// impl std::fmt::Display for TempInnerError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         <Self as Debug>::fmt(self, f)
//     }
// }
//
// impl std::error::Error for TempInnerError {}

/// This combines combines the functionalities of being able to send and receive mix packets.
pub trait GatewayTransceiver: GatewaySender + GatewayReceiver {
    fn gateway_identity(&self) -> identity::PublicKey;
}

/// This trait defines the functionality of sending `MixPacket` into the mixnet,
/// usually through a gateway.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GatewaySender {
    type Error: std::error::Error;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), Self::Error>;

    async fn batch_send_mix_packets(&mut self, packets: Vec<MixPacket>) -> Result<(), Self::Error> {
        for packet in packets {
            self.send_mix_packet(packet).await?;
        }
        Ok(())
    }
}

/// this trait defines the functionality of being able to correctly route
/// packets received from the mixnet, i.e. acks and 'proper' messages.
pub trait GatewayReceiver {
    type PacketRouter: GatewayPacketRouter;
}

pub trait GatewayPacketRouter {
    type Error: std::error::Error;

    // TODO: try to make it immutable
    fn route_received(&mut self, unwrapped_packets: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        let mut received_messages = Vec::new();
        let mut received_acks = Vec::new();

        // remember: gateway removes final layer of sphinx encryption and from the unwrapped
        // data he takes the SURB-ACK and first hop address.
        // currently SURB-ACKs are attached in EVERY packet, even cover, so this is always true
        let sphinx_ack_overhead = PacketSize::AckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN;
        let outfox_ack_overhead =
            PacketSize::OutfoxAckPacket.size() + MAX_NODE_ADDRESS_UNPADDED_LEN;

        for received_packet in unwrapped_packets {
            // note: if we ever fail to route regular outfox, it might be because I've removed a match on
            // `size == PacketSize::OutfoxRegularPacket.size() - outfox_ack_overhead` since it seemed
            // redundant given we have `size == PacketSize::OutfoxRegularPacket.plaintext_size() - outfox_ack_overhead`
            // and all the headers should have already be stripped at this point
            match received_packet.len() {
                n if n == PacketSize::AckPacket.plaintext_size() => {
                    trace!("received sphinx ack");
                    received_acks.push(received_packet);
                }

                n if n <= PacketSize::OutfoxAckPacket.plaintext_size() => {
                    // we don't know the real size of the payload, it could be anything <= 48 bytes
                    trace!("received outfox ack");
                    received_acks.push(received_packet);
                }

                n if n == PacketSize::RegularPacket.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received regular sphinx packet");
                    received_messages.push(received_packet);
                }

                n if n
                    == PacketSize::OutfoxRegularPacket.plaintext_size() - outfox_ack_overhead =>
                {
                    trace!("received regular outfox packet");
                    received_messages.push(received_packet);
                }

                n if n == PacketSize::ExtendedPacket8.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received extended8 packet");
                    received_messages.push(received_packet);
                }

                n if n == PacketSize::ExtendedPacket16.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received extended16 packet");
                    received_messages.push(received_packet);
                }

                n if n == PacketSize::ExtendedPacket32.plaintext_size() - sphinx_ack_overhead => {
                    trace!("received extended32 packet");
                    received_messages.push(received_packet);
                }

                n => {
                    // this can happen if other clients are not padding their messages
                    warn!("Received message of unexpected size. Probably from an outdated client... len: {n}");
                    received_messages.push(received_packet);
                }
            }
        }

        if !received_messages.is_empty() {
            trace!("routing {} received packets", received_messages.len());
            if let Err(err) = self.route_mixnet_messages(received_messages) {
                error!("failed to route received messages: {err}");
                return Err(err);
            }
        }

        if !received_acks.is_empty() {
            trace!("routing {} received acks", received_acks.len());
            if let Err(err) = self.route_acks(received_acks) {
                error!("failed to route received acks: {err}");
                return Err(err);
            }
        }

        Ok(())
    }

    fn route_mixnet_messages(&self, received_messages: Vec<Vec<u8>>) -> Result<(), Self::Error>;

    fn route_acks(&self, received_acks: Vec<Vec<u8>>) -> Result<(), Self::Error>;
}

// to allow for dynamic dispatch
impl<G: GatewayTransceiver + ?Sized> GatewayTransceiver for Box<G> {
    #[inline]
    fn gateway_identity(&self) -> identity::PublicKey {
        (**self).gateway_identity()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<G: GatewaySender + ?Sized + Send> GatewaySender for Box<G> {
    type Error = <G as GatewaySender>::Error;

    #[inline]
    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), Self::Error> {
        (**self).send_mix_packet(packet).await
    }

    #[inline]
    async fn batch_send_mix_packets(&mut self, packets: Vec<MixPacket>) -> Result<(), Self::Error> {
        (**self).batch_send_mix_packets(packets).await
    }
}

impl<G: GatewayReceiver + ?Sized> GatewayReceiver for Box<G> {
    type PacketRouter = <G as GatewayReceiver>::PacketRouter;
}

/// Gateway to which the client is connected through a socket.
/// Most likely through a websocket.
pub struct RemoteGateway<C, St> {
    gateway_client: GatewayClient<C, St>,
}

impl<C, St> RemoteGateway<C, St> {
    pub fn new(gateway_client: GatewayClient<C, St>) -> Self {
        Self { gateway_client }
    }
}

impl<C, St> GatewayTransceiver for RemoteGateway<C, St> {
    // type Error = GatewayClientError;
    fn gateway_identity(&self) -> identity::PublicKey {
        self.gateway_client.gateway_identity()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C, St> GatewaySender for RemoteGateway<C, St>
where
    C: Send,
    St: Send,
{
    type Error = GatewayClientError;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), GatewayClientError> {
        self.gateway_client.send_mix_packet(packet).await
    }

    async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), GatewayClientError> {
        self.gateway_client.batch_send_mix_packets(packets).await
    }
}

impl<C, St> GatewayReceiver for RemoteGateway<C, St> {
    type PacketRouter = nym_gateway_client::PacketRouter;
}

impl GatewayPacketRouter for nym_gateway_client::PacketRouter {
    type Error = ();

    fn route_mixnet_messages(&self, received_messages: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        todo!()
    }

    fn route_acks(&self, received_acks: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        todo!()
    }
}

/// Gateway running within the same process.
pub struct LocalGateway {
    // some channel or something
}

#[async_trait]
impl GatewayTransceiver for LocalGateway {
    // to be replaced with a concrete type once we define it, but for now I just want it to compile
    // type Error = MockGatewayError;

    fn gateway_identity(&self) -> identity::PublicKey {
        todo!()
    }

    async fn send_mix_packet(&mut self, _packet: MixPacket) -> Result<(), TempInnerError> {
        todo!()
    }

    async fn batch_send_mix_packets(
        &mut self,
        _packets: Vec<MixPacket>,
    ) -> Result<(), TempInnerError> {
        todo!()
    }
}

// if we ever decided to start writing unit tests... : )
pub struct MockGateway {
    dummy_identity: identity::PublicKey,
    sent: Vec<MixPacket>,
}

impl Default for MockGateway {
    fn default() -> Self {
        MockGateway {
            dummy_identity: "3ebjp1Fb9hdcS1AR6AZihgeJiMHkB5jjJUsvqNnfQwU7"
                .parse()
                .unwrap(),
            sent: vec![],
        }
    }
}

#[derive(Debug)]
pub struct MockGatewayError;

impl std::fmt::Display for MockGatewayError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl std::error::Error for MockGatewayError {}

#[async_trait]
impl GatewayTransceiver for MockGateway {
    fn gateway_identity(&self) -> identity::PublicKey {
        self.dummy_identity
    }
    // type Error = MockGatewayError;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), TempInnerError> {
        self.sent.push(packet);
        Ok(())
    }
}
