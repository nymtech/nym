// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use log::{debug, error};
use nym_credential_storage::storage::Storage as CredentialStorage;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::error::GatewayClientError;
use nym_gateway_client::GatewayClient;
pub use nym_gateway_client::{GatewayPacketRouter, PacketRouter};
use nym_gateway_requests::ClientRequest;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use std::fmt::Debug;
use std::os::raw::c_int as RawFd;
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
use futures::channel::oneshot;

// we need to type erase the error type since we can't have dynamic associated types alongside dynamic dispatch
#[derive(Debug, Error)]
#[error(transparent)]
pub struct ErasedGatewayError(Box<dyn std::error::Error + Send + Sync>);

fn erase_err<E: std::error::Error + Send + Sync + 'static>(err: E) -> ErasedGatewayError {
    ErasedGatewayError(Box::new(err))
}

/// This combines combines the functionalities of being able to send and receive mix packets.
#[async_trait]
pub trait GatewayTransceiver: GatewaySender + GatewayReceiver {
    fn gateway_identity(&self) -> ed25519::PublicKey;
    fn ws_fd(&self) -> Option<RawFd>;
    async fn send_client_request(
        &mut self,
        message: ClientRequest,
    ) -> Result<(), GatewayClientError>;

    /// Check if the websocket connection to the gateway is alive
    fn is_connection_alive(&self) -> bool;
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
        // allow for optimisation when sending multiple packets
        for packet in packets {
            self.send_mix_packet(packet).await?;
        }
        Ok(())
    }
}

/// this trait defines the functionality of being able to correctly route
/// packets received from the mixnet, i.e. acks and 'proper' messages.
pub trait GatewayReceiver {
    // ughhhh I really dislike this method, but couldn't come up wih anything better
    // ideally this would have been an associated type, but heh. we can't.
    fn set_packet_router(
        &mut self,
        _packet_router: PacketRouter,
    ) -> Result<(), ErasedGatewayError> {
        debug!("no-op packet router setup");
        Ok(())
    }
}

// to allow for dynamic dispatch
#[async_trait]
impl<G: GatewayTransceiver + ?Sized + Send> GatewayTransceiver for Box<G> {
    #[inline]
    fn gateway_identity(&self) -> ed25519::PublicKey {
        (**self).gateway_identity()
    }
    fn ws_fd(&self) -> Option<RawFd> {
        (**self).ws_fd()
    }

    async fn send_client_request(
        &mut self,
        message: ClientRequest,
    ) -> Result<(), GatewayClientError> {
        let _ = (**self).send_client_request(message.clone()).await?;
        log::debug!("Sent client request: {:?}", message);
        Ok(())
    }

    #[inline]
    fn is_connection_alive(&self) -> bool {
        (**self).is_connection_alive()
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
    fn set_packet_router(&mut self, packet_router: PacketRouter) -> Result<(), ErasedGatewayError> {
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

#[async_trait]
impl<C, St> GatewayTransceiver for RemoteGateway<C, St>
where
    C: DkgQueryClient + Send + Sync,
    St: CredentialStorage,
    <St as CredentialStorage>::StorageError: Send + Sync + 'static,
{
    fn gateway_identity(&self) -> ed25519::PublicKey {
        self.gateway_client.gateway_identity()
    }
    fn ws_fd(&self) -> Option<RawFd> {
        self.gateway_client.ws_fd()
    }

    async fn send_client_request(
        &mut self,
        message: ClientRequest,
    ) -> Result<(), GatewayClientError> {
        self.gateway_client.send_client_request(message).await
    }

    fn is_connection_alive(&self) -> bool {
        self.gateway_client.is_connection_alive()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C, St> GatewaySender for RemoteGateway<C, St>
where
    C: DkgQueryClient + Send + Sync,
    St: CredentialStorage,
    <St as CredentialStorage>::StorageError: Send + Sync + 'static,
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

impl<C, St> GatewayReceiver for RemoteGateway<C, St> {}

#[derive(Debug, Error)]
pub enum LocalGatewayError {
    #[error("attempted to set the packet router for the second time")]
    PacketRouterAlreadySet,

    #[error("failed to setup packet router - has the receiver been dropped?")]
    FailedPacketRouterSetup,
}

/// Gateway running within the same process.
#[cfg(not(target_arch = "wasm32"))]
pub struct LocalGateway {
    /// Identity of the locally managed gateway
    local_identity: ed25519::PublicKey,

    // 'sender' part
    /// Channel responsible for taking mix packets and forwarding them further into the further mixnet layers.
    packet_forwarder: nym_mixnet_client::forwarder::MixForwardingSender,

    // 'receiver' part
    packet_router_tx: Option<oneshot::Sender<PacketRouter>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl LocalGateway {
    pub fn new(
        local_identity: ed25519::PublicKey,
        packet_forwarder: nym_mixnet_client::forwarder::MixForwardingSender,
        packet_router_tx: oneshot::Sender<PacketRouter>,
    ) -> Self {
        LocalGateway {
            local_identity,
            packet_forwarder,
            packet_router_tx: Some(packet_router_tx),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod nonwasm_sealed {
    use super::*;

    #[async_trait]
    impl GatewayTransceiver for LocalGateway {
        fn gateway_identity(&self) -> ed25519::PublicKey {
            self.local_identity
        }
        fn ws_fd(&self) -> Option<RawFd> {
            None
        }

        async fn send_client_request(
            &mut self,
            _message: ClientRequest,
        ) -> Result<(), GatewayClientError> {
            Ok(())
        }

        fn is_connection_alive(&self) -> bool {
            // LocalGateway is always "connected" since it's in-process
            true
        }
    }

    #[async_trait]
    impl GatewaySender for LocalGateway {
        async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), ErasedGatewayError> {
            self.packet_forwarder
                .forward_packet(packet)
                .map_err(erase_err)
        }
    }

    impl GatewayReceiver for LocalGateway {
        fn set_packet_router(
            &mut self,
            packet_router: PacketRouter,
        ) -> Result<(), ErasedGatewayError> {
            let Some(packet_routex_tx) = self.packet_router_tx.take() else {
                return Err(erase_err(LocalGatewayError::PacketRouterAlreadySet));
            };

            packet_routex_tx
                .send(packet_router)
                .map_err(|_| erase_err(LocalGatewayError::FailedPacketRouterSetup))
        }
    }
}

// if we ever decided to start writing unit tests... : )
pub struct MockGateway {
    dummy_identity: ed25519::PublicKey,
    packet_router: Option<PacketRouter>,
    sent: Vec<MixPacket>,
}

impl Default for MockGateway {
    fn default() -> Self {
        MockGateway {
            dummy_identity: "3ebjp1Fb9hdcS1AR6AZihgeJiMHkB5jjJUsvqNnfQwU7"
                .parse()
                .unwrap(),
            packet_router: None,
            sent: vec![],
        }
    }
}

#[derive(Debug, Error)]
#[error("mock gateway error")]
pub struct MockGatewayError;

impl GatewayReceiver for MockGateway {
    // TODO: that's frustrating. can't do anything about the behaviour here since all the routing is in the `PacketRouter`...
    fn set_packet_router(&mut self, packet_router: PacketRouter) -> Result<(), ErasedGatewayError> {
        self.packet_router = Some(packet_router);
        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl GatewaySender for MockGateway {
    async fn send_mix_packet(&mut self, packet: MixPacket) -> Result<(), ErasedGatewayError> {
        self.sent.push(packet);
        Ok(())
    }
}

#[async_trait]
impl GatewayTransceiver for MockGateway {
    fn gateway_identity(&self) -> ed25519::PublicKey {
        self.dummy_identity
    }
    fn ws_fd(&self) -> Option<RawFd> {
        None
    }

    async fn send_client_request(
        &mut self,
        _message: ClientRequest,
    ) -> Result<(), GatewayClientError> {
        Ok(())
    }

    fn is_connection_alive(&self) -> bool {
        // MockGateway is always "connected" for testing purposes
        true
    }
}
