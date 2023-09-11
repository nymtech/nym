// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::channel::{mpsc, oneshot};
use log::{debug, error};
use nym_crypto::asymmetric::identity;
use nym_gateway_client::GatewayClient;
pub use nym_gateway_client::{GatewayPacketRouter, PacketRouter};
use nym_sphinx::forwarding::packet::MixPacket;
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
// can't define routing behaviour on the trait itself since GatewayClient will clone the packet router
// and send it to a `PartiallyDelegated` socket -> imo that should be redesigned...
pub trait GatewayReceiver {
    fn route_received(&mut self, plaintexts: Vec<Vec<u8>>) -> Result<(), ErasedGatewayError>;

    // ughhhh I really dislike this method, but couldn't come up wih anything better
    fn set_packet_router(&mut self, _packet_router: PacketRouter) {
        debug!("no-op packet router setup")
    }
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
    #[inline]
    fn route_received(&mut self, plaintexts: Vec<Vec<u8>>) -> Result<(), ErasedGatewayError> {
        (**self).route_received(plaintexts)
    }

    #[inline]
    fn set_packet_router(&mut self, packet_router: PacketRouter) {
        (**self).set_packet_router(packet_router)
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
    fn route_received(&mut self, plaintexts: Vec<Vec<u8>>) -> Result<(), ErasedGatewayError> {
        // self.gateway_client.set_packet_router(router)
        todo!()
    }
    // type PacketRouter = nym_gateway_client::PacketRouter;
}

/// Gateway running within the same process.
pub struct LocalGateway {
    /// Identity of the locally managed gateway
    local_identity: identity::PublicKey,

    // 'sender' part
    /// Channel responsible for taking mix packets and forwarding them further into the further mixnet layers.
    // TODO: get the type alias from the mixnet client crate
    packet_forwarder: mpsc::UnboundedSender<MixPacket>,

    // 'receiver' part
    packet_router_tx: Option<oneshot::Sender<PacketRouter>>,
}

impl Drop for LocalGateway {
    fn drop(&mut self) {
        error!("local gw is getting dropped");
    }
}

impl LocalGateway {
    pub fn new(
        local_identity: identity::PublicKey,
        packet_forwarder: mpsc::UnboundedSender<MixPacket>,
        packet_router_tx: oneshot::Sender<PacketRouter>,
    ) -> Self {
        LocalGateway {
            local_identity,
            packet_forwarder,
            packet_router_tx: Some(packet_router_tx),
        }
    }
}

impl GatewayTransceiver for LocalGateway {
    fn gateway_identity(&self) -> identity::PublicKey {
        self.local_identity
    }
}

#[async_trait]
impl GatewaySender for LocalGateway {
    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), ErasedGatewayError> {
        self.packet_forwarder
            .unbounded_send(packet)
            .map_err(|err| err.into_send_error())
            .map_err(erase_err)
    }
}

impl GatewayReceiver for LocalGateway {
    fn route_received(&mut self, plaintexts: Vec<Vec<u8>>) -> Result<(), ErasedGatewayError> {
        todo!()
        // println!("routing!");
        // let Some(ref packet_router) = self.packet_router else {
        //     todo!()
        // };
        // packet_router.route_received(plaintexts).map_err(erase_err)
    }

    fn set_packet_router(&mut self, packet_router: PacketRouter) {
        self.packet_router_tx
            .take()
            .expect("already used")
            .send(packet_router)
            .expect("TODO")
        // warn!("setting packet router");
        // self.packet_router = Some(packet_router)
    }
    // TODO: or just a getter?
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
