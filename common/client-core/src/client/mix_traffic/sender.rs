// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_gateway_client::error::GatewayClientError;
use nym_gateway_client::GatewayClient;
use nym_sphinx::forwarding::packet::MixPacket;
use std::fmt::{Debug, Formatter};

/// This trait defines the functionality of sending `MixPacket` into the mixnet,
/// usually through a gateway.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait MixnetSender {
    type Error: std::error::Error;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), Self::Error>;

    async fn batch_send_mix_packets(&mut self, packets: Vec<MixPacket>) -> Result<(), Self::Error> {
        for packet in packets {
            self.send_mix_packet(packet).await?;
        }
        Ok(())
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
impl<C, St> MixnetSender for RemoteGateway<C, St>
where
    C: Send,
    St: Send,
{
    type Error = GatewayClientError;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), Self::Error> {
        self.gateway_client.send_mix_packet(packet).await
    }

    async fn batch_send_mix_packets(&mut self, packets: Vec<MixPacket>) -> Result<(), Self::Error> {
        self.gateway_client.batch_send_mix_packets(packets).await
    }
}

/// Gateway running within the same process.
pub struct LocalGateway {
    // some channel or something
}

#[async_trait]
impl MixnetSender for LocalGateway {
    // to be replaced with a concrete type once we define it, but for now I just want it to compile
    type Error = MockGatewayError;

    async fn send_mix_packet(&mut self, _packet: MixPacket) -> Result<(), Self::Error> {
        todo!()
    }

    async fn batch_send_mix_packets(
        &mut self,
        _packets: Vec<MixPacket>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

// if we ever decided to start writing unit tests... : )
pub struct MockGateway {
    sent: Vec<MixPacket>,
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
impl MixnetSender for MockGateway {
    type Error = MockGatewayError;

    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), Self::Error> {
        self.sent.push(packet);
        Ok(())
    }
}
