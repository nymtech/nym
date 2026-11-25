// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{RouteNodeSelection, RouteSessionConfig};
use anyhow::Context;
use futures::channel::mpsc;
use nym_bandwidth_controller::requests::BandwidthControllerRequestSender;
use nym_client_core::client::key_manager::ClientKeys;
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::client::GatewayListeners;
use nym_gateway_client::{
    AcknowledgementReceiver, GatewayClient, MixnetMessageReceiver, PacketRouter,
};
use nym_network_monitor_route_records::NodeId;
use nym_task::ShutdownToken;
use nym_topology::{NymRouteProvider, RoutingNode};
use url::Url;

/// The gateway a route session runs through, as a route run names it.
#[derive(Debug, Clone, Copy)]
pub struct SessionGateway {
    pub node_id: NodeId,
    pub identity_key: ed25519::PublicKey,
}

/// A route session registered with its gateway, with bandwidth claimed and its mixnodes read.
pub struct RouteSession {
    config: RouteSessionConfig,
    gateway: SessionGateway,
    keys: ClientKeys,
    topology: NymRouteProvider,

    /// The mixnodes the session draws from, one list per layer.
    layers: [Vec<RoutingNode>; 3],

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
        let topology = read_topology(&config.nym_api_url, config.node_selection).await?;
        let layers = mix_layers(&topology, config.node_selection, gateway)?;
        let gateway_listeners = gateway_listeners(&topology, gateway)?;

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

/// Reads the network once through the client stack's nym-api topology provider: its default read for the active set, its extended read otherwise.
async fn read_topology(
    nym_api_url: &Url,
    node_selection: RouteNodeSelection,
) -> anyhow::Result<NymRouteProvider> {
    todo!("NymApiTopologyProvider with Topology::default(), use_extended_topology unless ActiveSet")
}

/// The selected mixnodes by layer: active nodes keep their layer, any other goes to layer `node_id % 3 + 1`.
fn mix_layers(
    topology: &NymRouteProvider,
    node_selection: RouteNodeSelection,
    gateway: SessionGateway,
) -> anyhow::Result<[Vec<RoutingNode>; 3]> {
    todo!(
        "the three lists for the selection, without the session's gateway; an empty layer is an error"
    )
}

/// The gateway's listeners as a default client derives them from the topology (design Decision 16).
fn gateway_listeners(
    topology: &NymRouteProvider,
    gateway: SessionGateway,
) -> anyhow::Result<GatewayListeners> {
    todo!(
        "find the gateway in the topology, then SelectedGateway::from_topology_node with the default selection"
    )
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
    todo!("GatewayClient::new with the session's identity and disabled credentials mode")
}
