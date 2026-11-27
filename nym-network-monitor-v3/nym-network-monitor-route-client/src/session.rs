// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{RouteNodeSelection, RouteSessionConfig};
use crate::packets_in_flight::PacketsInFlight;
use crate::poisson_ticker::PoissonTicker;
use anyhow::{Context, anyhow, bail, ensure};
use futures::StreamExt;
use futures::channel::mpsc;
use futures::future::join_all;
use nym_bandwidth_controller::requests::BandwidthControllerRequestSender;
use nym_client_core::client::key_manager::ClientKeys;
use nym_client_core::init::types::SelectedGateway;
use nym_client_core_config_types::GatewayConnection;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::client::config::GatewayClientConfig;
use nym_gateway_client::client::{GatewayConfig, GatewayListeners};
use nym_gateway_client::{
    AcknowledgementReceiver, GatewayClient, MixnetMessageReceiver, PacketRouter,
};
use nym_gateway_requests::ClientRequest;
use nym_network_monitor_route_records::NodeId;
use nym_network_monitor_route_records::batch::{Batch, BatchHeader};
use nym_sphinx::Node as SphinxNode;
use nym_sphinx::acknowledgements::identifier::recover_identifier;
use nym_sphinx::acknowledgements::surb_ack::SurbAck;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::cover::generate_loop_packet_on_route;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::params::SphinxKeyRotation;
use nym_sphinx::receiver::{MessageReceiver, SphinxMessageReceiver};
use nym_statistics_common::clients::ClientStatsSender;
use nym_task::ShutdownToken;
use nym_topology::{NymRouteProvider, Role, RoutingNode};
use rand::CryptoRng;
use rand::seq::IndexedRandom;
use std::mem;
use std::pin::pin;
use std::time::Duration;
use tokio::time::{Instant, sleep, timeout_at};
use tracing::warn;

/// The gateway a route session runs through, as a route run names it.
#[derive(Debug, Clone, Copy)]
pub struct SessionGateway {
    pub node_id: NodeId,
    pub identity_key: ed25519::PublicKey,
}

impl SessionGateway {
    /// The gateway's entry in the topology, looked up as a client looks up its egress.
    fn routing_node<'a>(&self, topology: &'a NymRouteProvider) -> anyhow::Result<&'a RoutingNode> {
        topology
            .egress_by_identity(self.identity_key)
            .context("the gateway is missing from the topology")
    }

    /// The gateway's listeners as a default client derives them from the topology.
    fn listeners(&self, topology: &NymRouteProvider) -> anyhow::Result<GatewayListeners> {
        let node = self.routing_node(topology)?;

        // the client's default gateway selection: no forced TLS, hostnames allowed
        let SelectedGateway::Remote {
            gateway_listeners, ..
        } = SelectedGateway::from_topology_node(node.clone(), false, false)?
        else {
            bail!("the client's gateway selection did not return a remote gateway");
        };
        Ok(gateway_listeners)
    }
}

/// A route session registered with its gateway, with bandwidth claimed and its mixnodes read.
pub struct RouteSession {
    config: RouteSessionConfig,
    gateway: SessionGateway,
    keys: ClientKeys,
    address: Recipient,
    topology: NymRouteProvider,
    layers: MixLayers,
    gateway_client: GatewayClient,
    payloads: MixnetMessageReceiver,
    acks: AcknowledgementReceiver,
    message_receiver: SphinxMessageReceiver,
    in_flight: PacketsInFlight,
    shutdown_token: ShutdownToken,
}

impl RouteSession {
    /// Opens a session through `gateway` over the run's `topology` as an ordinary client would; any failure is the session's error.
    pub async fn establish(
        config: RouteSessionConfig,
        topology: NymRouteProvider,
        gateway: SessionGateway,
        bandwidth_controller: BandwidthControllerRequestSender,
        shutdown_token: ShutdownToken,
    ) -> anyhow::Result<RouteSession> {
        let keys = ClientKeys::generate_new(&mut rand::rng());
        let address = Recipient::new(
            *keys.identity_keypair().public_key(),
            *keys.encryption_keypair().public_key(),
            gateway.identity_key,
        );
        let layers = MixLayers::new(&topology, config.node_selection, gateway)?;
        let gateway_listeners = gateway.listeners(&topology)?;

        let (ack_sender, acks) = mpsc::unbounded();
        let (payload_sender, payloads) = mpsc::unbounded();
        let packet_router = PacketRouter::new(ack_sender, payload_sender, shutdown_token.clone());

        let mut gateway_client = new_gateway_client(
            &keys,
            gateway,
            gateway_listeners,
            packet_router,
            bandwidth_controller,
            shutdown_token.clone(),
        );
        let _ = gateway_client
            .perform_initial_authentication()
            .await
            .context("registration with the gateway failed")?;
        gateway_client
            .claim_initial_bandwidth()
            .await
            .context("the gateway refused the bandwidth claim")?;
        gateway_client
            .start_listening_for_mixnet_messages()
            .context("could not start listening for returning packets")?;

        Ok(RouteSession {
            config,
            gateway,
            keys,
            address,
            topology,
            layers,
            gateway_client,
            payloads,
            acks,
            message_receiver: SphinxMessageReceiver::new(),
            in_flight: PacketsInFlight::default(),
            shutdown_token,
        })
    }

    /// Sends, receives what comes back, asks the gateway to forget the client and closes, giving the session's batch.
    pub async fn run(mut self) -> anyhow::Result<Batch> {
        self.send_packets().await?;
        self.receive_until_all_back().await;
        self.forget_and_close().await;
        Ok(self.into_batch())
    }

    /// Sends one Poisson stream at a client's combined rate for the session's drawn sending time.
    async fn send_packets(&mut self) -> anyhow::Result<()> {
        let mut ticker = PoissonTicker::new(self.config.traffic.combined_average_delay());
        let mut sending_end = pin!(sleep(self.config.draw_sending_time()));

        loop {
            tokio::select! {
                _ = &mut sending_end => return Ok(()),
                _ = ticker.tick() => self.send_next_packet().await?,
            }
        }
    }

    /// Sends the next packet through the gateway.
    async fn send_next_packet(&mut self) -> anyhow::Result<()> {
        let packet = self.next_packet()?;
        self.gateway_client
            .send_mix_packet(packet)
            .await
            .context("could not send a packet through the gateway")
    }

    /// Builds the next packet and its SURB-ack on independently drawn routes, and records them.
    fn next_packet(&mut self) -> anyhow::Result<MixPacket> {
        let packet_route = self.layers.draw_route(&mut rand::rng());
        let ack_route = self.layers.draw_route(&mut rand::rng());
        let identifier = self
            .in_flight
            .insert(packet_route.node_ids, ack_route.node_ids);

        let traffic = &self.config.traffic;
        let surb_ack = SurbAck::construct_legacy_on_route(
            &mut rand::rng(),
            &self.address,
            &self.keys.ack_key(),
            identifier,
            traffic.average_ack_delay,
            &ack_route.sphinx_route,
        )?;
        let packet = generate_loop_packet_on_route(
            &mut rand::rng(),
            &packet_route.sphinx_route,
            surb_ack,
            &identifier,
            &self.address,
            traffic.average_packet_delay,
            traffic.packet_size(),
            SphinxKeyRotation::from(self.topology.current_key_rotation()),
        )?;
        Ok(packet)
    }

    /// Receives what came back during sending, then any more until all is back or the straggler wait ends.
    async fn receive_until_all_back(&mut self) {
        let mut waiting_end = pin!(sleep(self.config.waiting_duration));

        while !self.in_flight.all_back() {
            tokio::select! {
                _ = &mut waiting_end => return,
                Some(payloads) = self.payloads.next() => self.receive_payloads(payloads),
                Some(acks) = self.acks.next() => self.receive_acks(acks),
            }
        }
    }

    /// Decrypts returning payloads as the client's receiver does and marks their packets' payloads back.
    fn receive_payloads(&mut self, payloads: Vec<Vec<u8>>) {
        for mut payload in payloads {
            match self.message_receiver.recover_plaintext_from_regular_packet(
                self.keys.encryption_keypair().private_key(),
                &mut payload,
            ) {
                Ok(content) => self.in_flight.payload_back(content),
                Err(err) => warn!("could not decrypt a returning payload: {err}"),
            }
        }
    }

    /// Recovers returning acks' identifiers and marks their packets' acks back.
    fn receive_acks(&mut self, acks: Vec<Vec<u8>>) {
        for ack in acks {
            match recover_identifier(&self.keys.ack_key(), &ack) {
                Some(identifier) => self.in_flight.ack_back(&identifier),
                None => warn!("could not recover the identifier of a returning ack"),
            }
        }
    }

    /// Asks the gateway to forget the client and closes the connection; a failure is only logged, as the records stand.
    async fn forget_and_close(&mut self) {
        let forget_me = ClientRequest::ForgetMe {
            client: true,
            stats: true,
        };
        if let Err(err) = self.gateway_client.send_client_request(forget_me).await {
            warn!("could not ask the gateway to forget the client: {err}");
        }
        if let Err(err) = self.gateway_client.close_connection().await {
            warn!("could not close the connection to the gateway: {err}");
        }
    }

    /// Closes the table of packets in flight into the session's batch.
    fn into_batch(mut self) -> Batch {
        Batch {
            header: BatchHeader {
                gateway_node_id: self.gateway.node_id,
                mixnet_epoch: self.topology.absolute_epoch_id(),
            },
            records: mem::take(&mut self.in_flight).into_records(),
        }
    }
}

impl Drop for RouteSession {
    fn drop(&mut self) {
        // before any field goes, so the gateway client's router never sends into a closed channel
        self.shutdown_token.cancel();
    }
}

/// What became of the session through one gateway of a run.
pub struct SessionOutcome {
    pub gateway: SessionGateway,

    /// The session's batch, or why it has none.
    pub batch: anyhow::Result<Batch>,
}

/// Runs one session through each gateway side by side over one reading of the network, all ending by the same deadline; an error means that reading failed.
pub async fn run_sessions(
    config: &RouteSessionConfig,
    gateways: Vec<SessionGateway>,
    bandwidth_controller: &BandwidthControllerRequestSender,
    shutdown_token: &ShutdownToken,
    run_timeout: Duration,
) -> anyhow::Result<Vec<SessionOutcome>> {
    let deadline = Instant::now() + run_timeout;

    // read once for the whole run, so its sessions name one epoch and draw from the same layers
    let topology = config.read_topology(deadline).await?;

    let sessions = gateways.into_iter().map(|gateway| {
        let session = RouteSession::establish(
            config.clone(),
            topology.clone(),
            gateway,
            bandwidth_controller.clone(),
            shutdown_token.child_token(),
        );
        async move {
            let batch = timeout_at(deadline, async { session.await?.run().await })
                .await
                .unwrap_or_else(|_| {
                    Err(anyhow!(
                        "the session did not finish before the run's deadline"
                    ))
                });
            SessionOutcome { gateway, batch }
        }
    });
    Ok(join_all(sessions).await)
}

/// The mixnodes a session draws from, by layer, and the gateway every route ends at.
struct MixLayers {
    layer1: Vec<RoutingNode>,
    layer2: Vec<RoutingNode>,
    layer3: Vec<RoutingNode>,
    gateway: SphinxNode,
}

impl MixLayers {
    /// The selected mixnodes by layer: active nodes keep their layer, any other goes to layer `node_id % 3 + 1`.
    fn new(
        topology: &NymRouteProvider,
        node_selection: RouteNodeSelection,
        gateway: SessionGateway,
    ) -> anyhow::Result<MixLayers> {
        let network = &topology.topology;
        let mut layers = MixLayers {
            layer1: network.nodes_with_role(Role::Layer1).cloned().collect(),
            layer2: network.nodes_with_role(Role::Layer2).cloned().collect(),
            layer3: network.nodes_with_role(Role::Layer3).cloned().collect(),
            gateway: gateway.routing_node(topology)?.into(),
        };

        let outside_active_set: Vec<&RoutingNode> = match node_selection {
            RouteNodeSelection::ActiveSet => Vec::new(),
            RouteNodeSelection::RewardedSet => network
                .nodes_with_role(Role::Standby)
                .filter(|node| node.supported_roles.mixnode)
                .collect(),
            RouteNodeSelection::AllMixnodes => network
                .mixnode_capable_nodes()
                .filter(|node| !network.rewarded_set().is_active_mixnode(&node.node_id))
                .collect(),
        };
        for node in outside_active_set {
            layers.layer_for(node.node_id).push(node.clone());
        }

        ensure!(
            !layers.layer1.is_empty() && !layers.layer2.is_empty() && !layers.layer3.is_empty(),
            "a mix layer has no node to route through"
        );
        Ok(layers)
    }

    /// The layer a node outside the active set is placed in: `node_id % 3 + 1`.
    fn layer_for(&mut self, node_id: NodeId) -> &mut Vec<RoutingNode> {
        match node_id % 3 {
            0 => &mut self.layer1,
            1 => &mut self.layer2,
            _ => &mut self.layer3,
        }
    }

    /// One mixnode drawn uniformly from each layer, in path order, then the gateway.
    fn draw_route<R: CryptoRng + ?Sized>(&self, rng: &mut R) -> DrawnRoute {
        let nodes = [&self.layer1, &self.layer2, &self.layer3].map(|layer| {
            // SAFETY: `new` refuses to build layers with an empty one
            #[allow(clippy::expect_used)]
            layer.choose(rng).expect("a mix layer is empty")
        });

        DrawnRoute {
            node_ids: nodes.map(|node| node.node_id),
            sphinx_route: nodes
                .into_iter()
                .map(SphinxNode::from)
                .chain([self.gateway.clone()])
                .collect(),
        }
    }
}

/// A route drawn for one packet or SURB-ack.
struct DrawnRoute {
    /// Node ids of the route's mixnodes, in path order.
    node_ids: [NodeId; 3],

    /// The route as the sphinx builders take it: the three mixnodes, then the gateway.
    sphinx_route: Vec<SphinxNode>,
}

/// The stock gateway client, with tickets not required and the caller's bandwidth controller sender.
fn new_gateway_client(
    keys: &ClientKeys,
    gateway: SessionGateway,
    gateway_listeners: GatewayListeners,
    packet_router: PacketRouter,
    bandwidth_controller: BandwidthControllerRequestSender,
    shutdown_token: ShutdownToken,
) -> GatewayClient {
    let config = GatewayClientConfig::new_default()
        .with_disabled_credentials_mode(true)
        .with_response_timeout(GatewayConnection::default().gateway_response_timeout);

    GatewayClient::new(
        config,
        GatewayConfig::new(gateway.identity_key, gateway_listeners),
        keys.identity_keypair(),
        None,
        packet_router,
        Box::new(bandwidth_controller),
        ClientStatsSender::new(None, shutdown_token.clone()),
        #[cfg(unix)]
        None,
        shutdown_token,
    )
}
