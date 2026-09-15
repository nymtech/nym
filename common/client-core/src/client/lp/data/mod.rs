// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, mpsc};

use crate::client::inbound_messages::InputMessageReceiver;
use crate::client::lp::data::handler::LpDataHandler;
use crate::client::lp::data::handler::inbound::ClientInbound;
use crate::client::lp::data::handler::outbound::{ClientOutbound, LpOutboundJobSender};
use crate::client::lp::data::handler::pipeline::{LpInboundPipeline, LpOutboundPipeline};
use crate::client::lp::data::listener::LpDataListener;
use crate::client::lp::data::shared::{LpGatewaySessions, SharedLpDataState};
use crate::client::received_buffer::ReceivedMessagesBuffer;
use crate::client::topology_control::TopologyAccessor;
use crate::config::Config;
use crate::error::ClientCoreError;

use nym_crypto::asymmetric::x25519;
use nym_lp::transport::LpDatagramChannel;
use nym_lp_gateway_client::LpGatewayDataClient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_sphinx::receiver::SphinxMessageReceiver;
use nym_task::ShutdownTracker;
use rand::rngs::OsRng;
use tokio::net::UdpSocket;
use tracing::{error, info};

pub(crate) const PACKET_BUFFER_SIZE: usize = 100;

pub mod handler;
mod listener;
pub mod shared;

pub struct LpDataSetup<D = UdpSocket>
where
    D: LpDatagramChannel,
{
    listener: LpDataListener<D>,

    handler: LpDataHandler,

    /// Where a caller that names its own destination submits work.
    ///
    /// Handed out by [`Self::job_sender`]; nothing in the client uses it yet, which is the point -
    /// it is the way in that does not go through [`InputMessage`].
    ///
    /// [`InputMessage`]: crate::client::inbound_messages::InputMessage
    job_tx: LpOutboundJobSender,

    /// Shutdown coordination
    shutdown: ShutdownTracker,
}

impl<D> LpDataSetup<D>
where
    // `'static` because the listener owning the socket is spawned as its own task
    D: LpDatagramChannel + 'static,
{
    /// Everything the LP data plane needs, from the parts a client already has.
    ///
    /// The two pipelines are built here rather than handed in, so that the one place that says how
    /// a message is wrapped is also the one that says how it is unwrapped.
    #[expect(clippy::too_many_arguments)]
    pub(crate) async fn new(
        config: &Config,
        encryption_keys: Arc<x25519::KeyPair>,
        gateway_sessions: LpGatewaySessions,
        topology_accessor: TopologyAccessor,
        received_buffer: ReceivedMessagesBuffer<SphinxMessageReceiver>,
        outbound_input_rx: InputMessageReceiver,
        gateway: NodeIdentity,
        shutdown: ShutdownTracker,
    ) -> Result<Self, ClientCoreError> {
        let socket =
            LpGatewayDataClient::<D>::bind(config.debug.lewes_protocol.data_socket_addr).await?;

        info!("LP data socket bound on {}", socket.local_address()?);

        // what both directions need; anything only one of them touches stays with that one
        let shared_state = Arc::new(SharedLpDataState::new(gateway_sessions));

        // the workers get one of each of these, cloned from them
        let outbound_pipeline =
            LpOutboundPipeline::new(OsRng, config.debug, topology_accessor, shared_state.clone());
        let inbound_pipeline = LpInboundPipeline::new(shared_state.clone(), encryption_keys);

        let (inbound_input_tx, inbound_input_rx) = mpsc::sync_channel(PACKET_BUFFER_SIZE);
        let (outbound_output_tx, outbound_output_rx) =
            tokio::sync::mpsc::channel(PACKET_BUFFER_SIZE);

        let listener = LpDataListener::new(
            socket,
            inbound_input_tx,
            outbound_output_rx,
            shutdown.clone_shutdown_token(),
        );

        // a pool of zero would silently drop everything handed to it
        let worker_count = config.debug.lewes_protocol.worker_threads.max(1);

        let inbound = ClientInbound::new(
            inbound_pipeline,
            inbound_input_rx,
            received_buffer,
            worker_count,
            &shutdown,
        );
        // the pipeline's own language, for anything that can name where its message goes
        let (job_tx, job_rx) = mpsc::channel();

        let outbound = ClientOutbound::new(
            outbound_pipeline,
            outbound_input_rx,
            job_rx,
            outbound_output_tx,
            shared_state,
            gateway,
            worker_count,
            &shutdown,
        );

        let handler = LpDataHandler::new(inbound, outbound, &shutdown);

        Ok(LpDataSetup {
            listener,
            handler,
            job_tx,
            shutdown,
        })
    }

    /// Submit work naming its own destination, rather than going through an [`InputMessage`].
    ///
    /// [`InputMessage`]: crate::client::inbound_messages::InputMessage
    pub fn job_sender(&self) -> LpOutboundJobSender {
        self.job_tx.clone()
    }

    pub fn start_tasks(mut self) {
        // Spawn the UDP data handler for LP data plane
        // The data handler listens on UDP port 51264 and processes LP-wrapped Sphinx packets
        // from registered clients. It decrypts the LP layer and forwards the Sphinx packets
        let shutdown_token = self.shutdown.clone_shutdown_token();
        let mut listener = self.listener;
        self.shutdown.try_spawn_named(
            async move {
                if let Err(err) = listener.run().await {
                    shutdown_token.cancel();
                    error!("LP data listener error: {err}");
                }
            },
            "LP::LpDataListener",
        );

        self.shutdown
            .try_spawn_named(async move { self.handler.run().await }, "LP::LpDataHandler");
    }
}
