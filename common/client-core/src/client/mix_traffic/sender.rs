// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_crypto::asymmetric::identity;
use nym_gateway_client::error::GatewayClientError;
use nym_gateway_client::GatewayClient;
use nym_sphinx::forwarding::packet::MixPacket;
use std::fmt::{Debug, Formatter};

#[derive(Debug)]
pub struct TempInnerError;

impl std::fmt::Display for TempInnerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

impl std::error::Error for TempInnerError {}

/// This trait defines the functionality of sending `MixPacket` into the mixnet,
/// usually through a gateway.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GatewaySender {
    // type Error: std::error::Error;
    fn gateway_identity(&self) -> identity::PublicKey;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), TempInnerError>;

    async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), TempInnerError> {
        for packet in packets {
            self.send_mix_packet(packet).await?;
        }
        Ok(())
    }
}

// to allow for dynamic dispatch
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<G: GatewaySender + ?Sized + Send> GatewaySender for Box<G> {
    fn gateway_identity(&self) -> identity::PublicKey {
        (**self).gateway_identity()
    }

    #[inline]
    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), TempInnerError> {
        (**self).send_mix_packet(packet).await
    }

    #[inline]
    async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), TempInnerError> {
        (**self).batch_send_mix_packets(packets).await
    }
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

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C, St> GatewaySender for RemoteGateway<C, St>
where
    C: Send,
    St: Send,
{
    // type Error = GatewayClientError;
    fn gateway_identity(&self) -> identity::PublicKey {
        self.gateway_client.gateway_identity()
    }

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), TempInnerError> {
        self.gateway_client
            .send_mix_packet(packet)
            .await
            .map_err(|_| todo!())
    }

    async fn batch_send_mix_packets(
        &mut self,
        packets: Vec<MixPacket>,
    ) -> Result<(), TempInnerError> {
        self.gateway_client
            .batch_send_mix_packets(packets)
            .await
            .map_err(|_| todo!())
    }
}

/// Gateway running within the same process.
pub struct LocalGateway {
    // some channel or something
}

#[async_trait]
impl GatewaySender for LocalGateway {
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
impl GatewaySender for MockGateway {
    fn gateway_identity(&self) -> identity::PublicKey {
        self.dummy_identity
    }
    // type Error = MockGatewayError;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), TempInnerError> {
        self.sent.push(packet);
        Ok(())
    }
}
