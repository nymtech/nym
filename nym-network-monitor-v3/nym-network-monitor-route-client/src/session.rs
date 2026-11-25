// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{RouteNodeSelection, RouteSessionConfig};
use anyhow::{Context, bail, ensure};
use futures::channel::mpsc;
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
use nym_network_monitor_route_records::NodeId;
use nym_statistics_common::clients::ClientStatsSender;
use nym_task::ShutdownToken;
use nym_topology::{NymRouteProvider, Role, RoutingNode};

/// The gateway a route session runs through, as a route run names it.
#[derive(Debug, Clone, Copy)]
pub struct SessionGateway {
    pub node_id: NodeId,
    pub identity_key: ed25519::PublicKey,
}

impl SessionGateway {
    /// The gateway's listeners as a default client derives them from the topology (design Decision 16).
    fn listeners(&self, topology: &NymRouteProvider) -> anyhow::Result<GatewayListeners> {
        let node = topology
            .egress_by_identity(self.identity_key)
            .context("the gateway is missing from the topology")?;

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
    topology: NymRouteProvider,
    layers: MixLayers,
    gateway_client: GatewayClient,
    payloads: MixnetMessageReceiver,
    acks: AcknowledgementReceiver,
    shutdown_token: ShutdownToken,
}

impl RouteSession {
    /// Opens a session through `gateway` as an ordinary client would; any failure is the session's error.
    pub async fn establish(
        config: RouteSessionConfig,
        gateway: SessionGateway,
        bandwidth_controller: BandwidthControllerRequestSender,
    ) -> anyhow::Result<RouteSession> {
        let keys = ClientKeys::generate_new(&mut rand::rng());
        let topology = config.read_topology().await?;
        let layers = MixLayers::new(&topology, config.node_selection)?;
        let gateway_listeners = gateway.listeners(&topology)?;

        let shutdown_token = ShutdownToken::default();
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

        Ok(RouteSession {
            config,
            gateway,
            keys,
            topology,
            layers,
            gateway_client,
            payloads,
            acks,
            shutdown_token,
        })
    }
}

/// The mixnodes a session draws from, by layer.
struct MixLayers {
    layer1: Vec<RoutingNode>,
    layer2: Vec<RoutingNode>,
    layer3: Vec<RoutingNode>,
}

impl MixLayers {
    /// The selected mixnodes by layer: active nodes keep their layer, any other goes to layer `node_id % 3 + 1`.
    fn new(
        topology: &NymRouteProvider,
        node_selection: RouteNodeSelection,
    ) -> anyhow::Result<MixLayers> {
        let network = &topology.topology;
        let mut layers = MixLayers {
            layer1: network.nodes_with_role(Role::Layer1).cloned().collect(),
            layer2: network.nodes_with_role(Role::Layer2).cloned().collect(),
            layer3: network.nodes_with_role(Role::Layer3).cloned().collect(),
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
