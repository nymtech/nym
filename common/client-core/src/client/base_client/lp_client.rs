// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A client that talks to the network over the Lewes Protocol only.
//!
//! The counterpart of [`BaseClientBuilder`] for LP: it starts no gateway websocket, no mix traffic,
//! no cover traffic and no legacy topology. The startup steps the two have in common are
//! [`BaseClientBuilder`]'s own helpers, called from here.

use crate::client::base_client::BaseClientBuilder;
use crate::client::base_client::storage::MixnetClientStorage;
use crate::client::inbound_messages::{InputMessage, InputMessageReceiver, InputMessageSender};
use crate::client::lp::control::LpClientControlSetup;
use crate::client::lp::data::LpDataSetup;
use crate::client::lp::data::shared::LpGatewaySessions;
use crate::client::topology_control::{
    LpTopologyAccessor, LpTopologyProvider, LpTopologyRefresher, NymApiLpTopologyProvider,
    TopologyRefresherConfig,
};
use crate::config::{self, Config};
use crate::error::ClientCoreError;
use crate::init::types::GatewaySetup;
use futures::Stream;
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_crypto::hkdf::DerivationMaterial;
use nym_sphinx::addressing::clients::Recipient;
use nym_task::ShutdownTracker;
use nym_validator_client::{UserAgent, nyxd::contract_traits::DkgQueryClient};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

/// Where messages go to be sent over LP.
#[derive(Clone)]
pub struct LpClientInput {
    pub input_sender: InputMessageSender,
}

impl LpClientInput {
    #[expect(clippy::result_large_err)]
    pub async fn send(
        &self,
        message: InputMessage,
    ) -> Result<(), mpsc::error::SendError<InputMessage>> {
        self.input_sender.send(message).await
    }
}

pub struct LpBaseClient {
    pub address: Recipient,
    pub identity_keys: Arc<ed25519::KeyPair>,
    pub client_input: LpClientInput,

    /// The messages this client receives, one per item, waiting to be read through this client's
    /// [`Stream`] impl.
    ///
    /// Kept private so it lives exactly as long as the client: nothing can drop it while the
    /// client still runs.
    output_receiver: ReceiverStream<Vec<u8>>,

    /// Nothing on the LP path spends bandwidth yet; this keeps the controller reachable for
    /// whatever will.
    pub bandwidth_provider: Box<dyn BandwidthTicketProvider>,

    pub shutdown_handle: ShutdownTracker,
}

/// The messages this client receives, one per item.
///
/// Ends once the client has shut down and everything received before that has been read.
impl Stream for LpBaseClient {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.output_receiver).poll_next(cx)
    }
}

pub struct LpBaseClientBuilder<C, S: MixnetClientStorage> {
    config: Config,
    client_store: S,
    // when present, used to build a nyxd-backed public data fetcher for the bandwidth controller
    dkg_query_client: Option<C>,

    // Optional API URLs for domain fronting support
    nym_api_urls: Option<Vec<nym_network_defaults::ApiUrl>>,

    custom_topology_provider: Option<Box<dyn LpTopologyProvider>>,
    custom_bandwidth_provider: Option<Box<dyn BandwidthTicketProvider>>,
    shutdown: Option<ShutdownTracker>,
    user_agent: Option<UserAgent>,

    setup_method: GatewaySetup,

    derivation_material: Option<DerivationMaterial>,
}

impl<C, S> LpBaseClientBuilder<C, S>
where
    S: MixnetClientStorage + 'static,
    C: DkgQueryClient + 'static,
{
    pub fn new(
        base_config: Config,
        client_store: S,
        dkg_query_client: Option<C>,
    ) -> LpBaseClientBuilder<C, S> {
        LpBaseClientBuilder {
            config: base_config,
            client_store,
            dkg_query_client,
            nym_api_urls: None,
            custom_topology_provider: None,
            custom_bandwidth_provider: None,
            shutdown: None,
            user_agent: None,
            setup_method: GatewaySetup::MustLoad { gateway_id: None },
            derivation_material: None,
        }
    }

    #[must_use]
    pub fn with_derivation_material(
        mut self,
        derivation_material: Option<DerivationMaterial>,
    ) -> Self {
        self.derivation_material = derivation_material;
        self
    }

    /// Set Nym API URLs for domain fronting support.
    #[must_use]
    pub fn with_nym_api_urls(mut self, nym_api_urls: Vec<nym_network_defaults::ApiUrl>) -> Self {
        self.nym_api_urls = Some(nym_api_urls);
        self
    }

    /// Only the chosen gateway's identity is used: the LP control plane handshakes and registers
    /// with it itself, so nothing else from the stored registration matters here.
    #[must_use]
    pub fn with_gateway_setup(mut self, setup: GatewaySetup) -> Self {
        self.setup_method = setup;
        self
    }

    #[must_use]
    pub fn with_topology_provider(mut self, provider: Box<dyn LpTopologyProvider>) -> Self {
        self.custom_topology_provider = Some(provider);
        self
    }

    #[must_use]
    pub fn with_custom_bandwidth_provider(
        mut self,
        bandwidth_provider: Box<dyn BandwidthTicketProvider>,
    ) -> Self {
        self.custom_bandwidth_provider = Some(bandwidth_provider);
        self
    }

    #[must_use]
    pub fn with_shutdown(mut self, shutdown: ShutdownTracker) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    #[must_use]
    pub fn with_user_agent(mut self, user_agent: UserAgent) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Fill the LP view once, then keep it fresh.
    ///
    /// Fails if the LP network cannot be routed through: a client that believes it has LP and
    /// silently drops everything handed to it is worse than one that does not start.
    async fn start_lp_topology_refresher(
        topology_provider: Box<dyn LpTopologyProvider>,
        topology_config: config::Topology,
        topology_accessor: LpTopologyAccessor,
        shutdown_tracker: &ShutdownTracker,
    ) -> Result<(), ClientCoreError> {
        let mut refresher = LpTopologyRefresher::new(
            TopologyRefresherConfig::new(topology_config.topology_refresh_rate),
            topology_accessor.clone(),
            topology_provider,
        );

        info!("Obtaining initial LP network topology");
        refresher.try_refresh().await;

        if let Err(err) = topology_accessor.ensure_is_routable() {
            error!(
                "the LP network is not routable - check that enough nodes have published LP \
                 details: {err}"
            );
            return Err(ClientCoreError::InsufficientNetworkTopology(err));
        }

        if topology_config.disable_refreshing {
            info!("The background LP topology refresher is not going to be started");
            return Ok(());
        }

        info!("Starting LP topology refresher...");
        shutdown_tracker.try_spawn_named_with_shutdown(
            async move { refresher.run().await },
            "LpTopologyRefresher",
        );

        Ok(())
    }

    /// The LP path in full: the control plane, then the tasks that carry data over it.
    ///
    /// The home gateway is dialled here, and waited for, so that a client knows on the way up
    /// whether it has an LP session. The path itself is built either way; what a failed dial costs
    /// is every message handed to that path, until something asks for a session again.
    #[expect(clippy::too_many_arguments)]
    async fn build_lp_tasks(
        config: &Config,
        self_address: Recipient,
        identity_keys: Arc<ed25519::KeyPair>,
        encryption_keys: Arc<x25519::KeyPair>,
        topology_accessor: LpTopologyAccessor,
        input_receiver: InputMessageReceiver,
        received_tx: mpsc::Sender<Vec<u8>>,
        shutdown_tracker: &ShutdownTracker,
    ) -> Result<(), ClientCoreError> {
        let sessions = LpGatewaySessions::default();

        // the control plane first: nothing can ask for a session before there is something to ask
        let control = LpClientControlSetup::new(
            &config.debug.lewes_protocol,
            topology_accessor.clone(),
            sessions.clone(),
            identity_keys,
            *encryption_keys.public_key(),
            shutdown_tracker.clone_shutdown_token(),
        )?;

        // the home gateway, established before anything can try to send over it. Awaited rather
        // than hinted because this is the one dial nothing else would raise: the data plane drops
        // what it cannot address rather than asking for a session.
        let gateway = self_address.gateway();
        match control.dialer().ensure_session(gateway).await {
            Ok(address) => {
                info!("LP session with gateway {gateway} is ready, sending to {address}")
            }
            Err(err) => error!(
                "no LP session with gateway {gateway}: {err}. Anything sent over LP will be dropped"
            ),
        }

        LpDataSetup::<UdpSocket>::new(
            config,
            encryption_keys,
            sessions,
            topology_accessor,
            received_tx,
            input_receiver,
            gateway,
            shutdown_tracker.clone(),
        )
        .await?
        .start_tasks();

        Ok(())
    }

    pub async fn start(self) -> Result<LpBaseClient, ClientCoreError> {
        info!("Starting LP client");

        let init_res = BaseClientBuilder::<C, S>::initialise_keys_and_gateway(
            self.setup_method,
            self.client_store.key_store(),
            self.client_store.gateway_details_store(),
            self.derivation_material,
        )
        .await?;

        let (_, credential_store, _) = self.client_store.into_runtime_stores();

        let shutdown_tracker = match self.shutdown {
            Some(parent_tracker) => parent_tracker,
            None => nym_task::create_sdk_shutdown_tracker()?,
        };

        let self_address = init_res.client_address();
        let encryption_keys = init_res.client_keys.encryption_keypair();
        let identity_keys = init_res.client_keys.identity_keypair();

        let bandwidth_provider = BaseClientBuilder::<C, S>::start_bandwidth_controller(
            self.custom_bandwidth_provider,
            self.dkg_query_client,
            credential_store,
            &shutdown_tracker,
        );

        let topology_provider = match self.custom_topology_provider {
            Some(provider) => provider,
            None => Box::new(NymApiLpTopologyProvider::new(
                self.config.debug.topology,
                BaseClientBuilder::<C, S>::construct_nym_api_client(
                    self.nym_api_urls.as_ref(),
                    &self.config,
                    self.user_agent,
                )?,
            )),
        };

        // the LP view has to be there before the home gateway is dialled, since that dial resolves
        // the gateway against it
        let topology_accessor = LpTopologyAccessor::new();
        Self::start_lp_topology_refresher(
            topology_provider,
            self.config.debug.topology,
            topology_accessor.clone(),
            &shutdown_tracker,
        )
        .await?;

        let (input_sender, input_receiver) = tokio::sync::mpsc::channel::<InputMessage>(1);
        let (output_sender, output_receiver) = mpsc::channel(10_000);

        Self::build_lp_tasks(
            &self.config,
            self_address,
            identity_keys.clone(),
            encryption_keys,
            topology_accessor,
            input_receiver,
            output_sender,
            &shutdown_tracker,
        )
        .await?;

        info!("LP client startup finished, the address of this client is: {self_address}");

        Ok(LpBaseClient {
            address: self_address,
            identity_keys,
            client_input: LpClientInput { input_sender },
            output_receiver: ReceiverStream::new(output_receiver),
            bandwidth_provider,
            shutdown_handle: shutdown_tracker,
        })
    }
}
