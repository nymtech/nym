// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::embedded_clients::{LocalEmbeddedClientHandle, MessageRouter};
use crate::node::client_handling::websocket::message_receiver::{
    MixMessageReceiver, MixMessageSender,
};
use crate::GatewayError;
use async_trait::async_trait;
use futures::channel::{mpsc, oneshot};
use nym_authenticator::Authenticator;
use nym_crypto::asymmetric::ed25519;
use nym_ip_packet_router::error::IpPacketRouterError;
use nym_ip_packet_router::IpPacketRouter;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_network_requester::error::NetworkRequesterError;
use nym_network_requester::NRServiceProviderBuilder;
use nym_sdk::mixnet::Recipient;
use nym_sdk::{GatewayTransceiver, LocalGateway, PacketRouter};
use nym_task::TaskClient;
use std::fmt::Display;
use tokio::task::JoinHandle;
use tracing::error;

pub trait LocalRecipient {
    fn address(&self) -> Recipient;
}

impl LocalRecipient for nym_network_requester::core::OnStartData {
    fn address(&self) -> Recipient {
        self.address
    }
}

impl LocalRecipient for nym_ip_packet_router::OnStartData {
    fn address(&self) -> Recipient {
        self.address
    }
}

impl LocalRecipient for nym_authenticator::OnStartData {
    fn address(&self) -> Recipient {
        self.address
    }
}

#[async_trait]
pub trait RunnableServiceProvider {
    const NAME: &'static str;

    type OnStartData: LocalRecipient;
    type Error;
    async fn run_service_provider(self) -> Result<(), Self::Error>;
}

#[async_trait]
impl RunnableServiceProvider for NRServiceProviderBuilder {
    const NAME: &'static str = "network requester";
    type OnStartData = nym_network_requester::core::OnStartData;
    type Error = NetworkRequesterError;

    async fn run_service_provider(self) -> Result<(), Self::Error> {
        self.run_service_provider().await
    }
}

#[async_trait]
impl RunnableServiceProvider for IpPacketRouter {
    const NAME: &'static str = "ip router";
    type OnStartData = nym_ip_packet_router::OnStartData;
    type Error = IpPacketRouterError;

    async fn run_service_provider(self) -> Result<(), Self::Error> {
        self.run_service_provider().await
    }
}

#[async_trait]
impl RunnableServiceProvider for Authenticator {
    const NAME: &'static str = "authenticator";
    type OnStartData = nym_authenticator::OnStartData;
    type Error = nym_authenticator::error::AuthenticatorError;

    async fn run_service_provider(self) -> Result<(), Self::Error> {
        self.run_service_provider().await
    }
}

pub struct ServiceProviderBeingBuilt<T: RunnableServiceProvider> {
    on_start_rx: oneshot::Receiver<T::OnStartData>,
    sp_builder: T,
    sp_message_router_builder: SpMessageRouterBuilder,
}

pub struct StartedServiceProvider<T: RunnableServiceProvider> {
    pub sp_join_handle: JoinHandle<()>,
    pub message_router_join_handle: JoinHandle<()>,
    pub on_start_data: T::OnStartData,
    pub handle: LocalEmbeddedClientHandle,
}

impl<T> ServiceProviderBeingBuilt<T>
where
    T: RunnableServiceProvider + Send + Sync + 'static,
    T::Error: Display + Send + Sync + 'static,
{
    pub(crate) fn new(
        on_start_rx: oneshot::Receiver<T::OnStartData>,
        sp_builder: T,
        sp_message_router_builder: SpMessageRouterBuilder,
    ) -> Self {
        ServiceProviderBeingBuilt {
            on_start_rx,
            sp_builder,
            sp_message_router_builder,
        }
    }

    pub async fn start_service_provider(
        mut self,
    ) -> Result<StartedServiceProvider<T>, GatewayError> {
        let sp_join_handle = tokio::task::spawn(async move {
            if let Err(err) = self.sp_builder.run_service_provider().await {
                error!(
                    "the {} service provider encountered an error: {err}",
                    T::NAME
                )
            }
        });

        // TODO: if something is blocking during SP startup, the below will wait forever
        // we need to introduce additional timeouts here.
        let on_start_data = self
            .on_start_rx
            .await
            .map_err(|_| GatewayError::ServiceProviderStartupFailure { typ: T::NAME })?;

        // this should be instantaneous since the data is sent on this channel before the on start is called;
        // the failure should be impossible
        let Ok(Some(packet_router)) = self.sp_message_router_builder.router_receiver.try_recv()
        else {
            return Err(GatewayError::ServiceProviderStartupFailure { typ: T::NAME });
        };

        let mix_sender = self.sp_message_router_builder.mix_sender();
        let message_router_join_handle = self
            .sp_message_router_builder
            .start_message_router(packet_router);

        Ok(StartedServiceProvider {
            sp_join_handle,
            message_router_join_handle,
            handle: LocalEmbeddedClientHandle::new(on_start_data.address(), mix_sender),
            on_start_data,
        })
    }
}

pub struct ExitServiceProviders {
    pub(crate) network_requester: ServiceProviderBeingBuilt<NRServiceProviderBuilder>,
    pub(crate) ip_router: ServiceProviderBeingBuilt<IpPacketRouter>,
}

impl ExitServiceProviders {
    pub async fn start_service_providers(
        self,
    ) -> Result<
        (
            StartedServiceProvider<NRServiceProviderBuilder>,
            StartedServiceProvider<IpPacketRouter>,
        ),
        GatewayError,
    > {
        let started_nr = self.network_requester.start_service_provider().await?;
        let started_ipr = self.ip_router.start_service_provider().await?;

        Ok((started_nr, started_ipr))
    }
}

pub struct SpMessageRouterBuilder {
    mix_sender: Option<MixMessageSender>,
    mix_receiver: MixMessageReceiver,
    router_receiver: oneshot::Receiver<PacketRouter>,
    gateway_transceiver: Option<LocalGateway>,
    shutdown: TaskClient,
}

impl SpMessageRouterBuilder {
    pub(crate) fn new(
        node_identity: ed25519::PublicKey,
        forwarding_channel: MixForwardingSender,
        shutdown: TaskClient,
    ) -> Self {
        let (mix_sender, mix_receiver) = mpsc::unbounded();
        let (router_tx, router_rx) = oneshot::channel();

        let transceiver = LocalGateway::new(node_identity, forwarding_channel, router_tx);

        SpMessageRouterBuilder {
            mix_sender: Some(mix_sender),
            mix_receiver,
            router_receiver: router_rx,
            gateway_transceiver: Some(transceiver),
            shutdown,
        }
    }

    #[allow(clippy::expect_used)]
    pub(crate) fn gateway_transceiver(&mut self) -> Box<dyn GatewayTransceiver + Send + Sync> {
        Box::new(
            self.gateway_transceiver
                .take()
                .expect("attempting to use the same gateway transceiver twice"),
        )
    }

    #[allow(clippy::expect_used)]
    fn mix_sender(&mut self) -> MixMessageSender {
        self.mix_sender
            .take()
            .expect("attempting to use the same mix sender twice")
    }

    fn start_message_router(self, packet_router: PacketRouter) -> JoinHandle<()> {
        MessageRouter::new(self.mix_receiver, packet_router).start_with_shutdown(self.shutdown)
    }
}
