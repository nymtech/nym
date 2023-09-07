// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{error, trace, warn};
use nym_crypto::asymmetric::identity;
use nym_gateway_client::error::GatewayClientError;
use nym_gateway_client::{
    AcknowledgementSender, GatewayClient, GatewayPacketRouter, MixnetMessageSender,
};
use nym_sphinx::addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::PacketSize;
use std::fmt::{Debug, Formatter};
use thiserror::Error;

// #[derive(Debug)]
// pub struct TempInnerError;

// impl std::fmt::Display for TempInnerError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         <Self as Debug>::fmt(self, f)
//     }
// }
//
// impl std::error::Error for TempInnerError {}

// we need to erase the error type since we can't have dynamic associated types in dynamic dispatch
#[derive(Debug, Error)]
#[error(transparent)]
pub struct ErasedGatewayError(Box<dyn std::error::Error + Send + Sync>);

fn erase_err<E: std::error::Error + Send + Sync + 'static>(err: E) -> ErasedGatewayError {
    ErasedGatewayError(Box::new(err))
}

/// This combines combines the functionalities of being able to send and receive mix packets.
pub trait GatewayTransceiver: GatewaySender + GatewayReceiver {
    fn gateway_identity(&self) -> identity::PublicKey;
}

/// This trait defines the functionality of sending `MixPacket` into the mixnet,
/// usually through a gateway.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GatewaySender {
    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), ErasedGatewayError>;

    async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), ErasedGatewayError> {
        for packet in packets {
            self.send_mix_packet(packet).await?;
        }
        Ok(())
    }
}

/// this trait defines the functionality of being able to correctly route
/// packets received from the mixnet, i.e. acks and 'proper' messages.
pub trait GatewayReceiver {
    // type PacketRouter: GatewayPacketRouter;
}

// to allow for dynamic dispatch
impl<G: GatewayTransceiver + ?Sized + Send> GatewayTransceiver for Box<G> {
    #[inline]
    fn gateway_identity(&self) -> identity::PublicKey {
        (**self).gateway_identity()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<G: GatewaySender + ?Sized + Send> GatewaySender for Box<G> {
    #[inline]
    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), ErasedGatewayError> {
        (**self).send_mix_packet(packet).await
    }

    #[inline]
    async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), ErasedGatewayError> {
        (**self).batch_send_mix_packets(packets).await
    }
}

impl<G: GatewayReceiver + ?Sized> GatewayReceiver for Box<G> {
    // type PacketRouter = <G as GatewayReceiver>::PacketRouter;
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

impl<C, St> GatewayTransceiver for RemoteGateway<C, St>
where
    C: Send,
    St: Send,
{
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
    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), ErasedGatewayError> {
        self.gateway_client
            .send_mix_packet(packet)
            .await
            .map_err(erase_err)
    }

    async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), ErasedGatewayError> {
        self.gateway_client
            .batch_send_mix_packets(packets)
            .await
            .map_err(erase_err)
    }
}

impl<C, St> GatewayReceiver for RemoteGateway<C, St> {
    // type PacketRouter = nym_gateway_client::PacketRouter;
}

/// Gateway running within the same process.
pub struct LocalGateway {
    local_identity: identity::PublicKey,
    // 'sender' part
    // some channel or something

    // 'receiver' part
    // ack_forwarder: AcknowledgementSender,
    // mixnet_message_forwarder: MixnetMessageSender,
}

impl LocalGateway {
    pub fn new(local_identity: identity::PublicKey) -> Self {
        LocalGateway { local_identity }
    }
}

impl GatewayTransceiver for LocalGateway {
    fn gateway_identity(&self) -> identity::PublicKey {
        self.local_identity
    }
}

#[async_trait]
impl GatewaySender for LocalGateway {
    async fn send_mix_packet(&mut self, _packet: MixPacket) -> Result<(), ErasedGatewayError> {
        println!("here we are supposed to be sending a mix packet");
        Ok(())
    }

    async fn batch_send_mix_packets(
        &mut self,
        _packets: Vec<MixPacket>,
    ) -> Result<(), ErasedGatewayError> {
        println!(
            "here we are supposed to be sending {} mix packets",
            _packets.len()
        );
        Ok(())
    }
}

impl GatewayReceiver for LocalGateway {}

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

// #[async_trait]
// impl GatewayTransceiver for MockGateway {
//     fn gateway_identity(&self) -> identity::PublicKey {
//         self.dummy_identity
//     }
//     // type Error = MockGatewayError;
//
//     async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), TempInnerError> {
//         self.sent.push(packet);
//         Ok(())
//     }
// }
