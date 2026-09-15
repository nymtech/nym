// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Reading a node's keys, ports and roles off its own http api.
//!
//! Only the manual `test-*` commands use this. A real run needs none of it: the orchestrator has
//! already described every node it hands out, so an assignment arrives complete. Here there is no
//! orchestrator, and requiring an operator to copy six keys and ports onto a command line is both
//! tedious and a way to spend an afternoon debugging a probe that was aimed with a stale sphinx key.
//!
//! Deliberately NOT shared with the orchestrator's refresher, which reads the same endpoints: that
//! one starts from a contract bond, verifies the node against the identity committed on chain, and
//! folds the result into a storage row. This one starts from an address typed by hand and has no
//! chain to check anything against.

use crate::agent::tested_node::{TestedGatewayDetails, TestedNodeDetails};
use anyhow::{Context, bail};
use nym_bin_common::bin_info;
use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::DEFAULT_MIX_LISTENING_PORT;
use nym_node_requests::api::client::NymNodeApiClientExt;
use nym_node_requests::api::helpers::NymNodeApiClientRetriever;
use nym_node_requests::api::v1::node::models::NodeRoles;
use nym_sphinx_params::SphinxKeyRotation;
use std::net::SocketAddr;
use tracing::info;

/// The node id every query here reports itself under.
///
/// The client only uses it to label its errors, and a manually targeted node has no id: it is not
/// being looked up in the contract, it is being addressed directly.
const UNIDENTIFIED_NODE: u32 = 0;

/// What a manual run learned about the node it is aimed at.
pub(crate) struct DiscoveredNode {
    /// The mixnet listener and the keys reaching it, which is all a mixnode probe of either kind
    /// needs.
    pub(crate) mixnet: TestedNodeDetails,

    /// The node's ed25519 identity, as it reports and signs for it. A gateway probe authenticates the
    /// registration handshake against this.
    pub(crate) identity: ed25519::PublicKey,

    /// What the node says it can do, which is what decides whether a given probe is applicable at
    /// all.
    pub(crate) roles: NodeRoles,

    /// Port of the plain client websocket, present only for a gateway-capable node.
    pub(crate) clients_ws_port: Option<u16>,
}

impl DiscoveredNode {
    /// Queries `host` and reads back everything a probe needs.
    ///
    /// `http_port` is only needed when the node serves its api somewhere non-standard; the client
    /// probes the usual ports otherwise. The host information's SIGNATURE is verified, so the keys a
    /// probe is about to be aimed with are known to belong to the identity that reported them rather
    /// than to whatever answered on the port.
    pub(crate) async fn query(host: &str, http_port: Option<u16>) -> anyhow::Result<Self> {
        info!("querying {host} for its keys, ports and roles");

        let client = NymNodeApiClientRetriever::new(bin_info!())
            .with_verify_host_information()
            .with_custom_port(http_port)
            .get_client(host, UNIDENTIFIED_NODE)
            .await
            .with_context(|| format!("failed to reach the http api of {host}"))?;

        let api_client = client.client;
        let host_info = client
            .host_information
            .context("the node returned no host information")?;

        // a node with no noise key is too old to be probed at all: every leg of every probe reaches
        // its mixnet listener over Noise
        let noise_key = host_info
            .keys
            .x25519_versioned_noise
            .context("the node announced no noise key, so it is too old to be probed")?
            .x25519_pubkey;

        // announced rather than the address we dialled: a probe's return hop has to name an address
        // the node will actually send from, and its http api may well not be on the same interface
        let mut known_ips = host_info
            .ip_address
            .iter()
            .map(|ip| ip.to_canonical())
            .collect::<Vec<_>>();
        known_ips.sort_unstable();
        known_ips.dedup();

        let address = SocketAddr::new(
            *known_ips
                .first()
                .context("the node announced no ip addresses")?,
            api_client
                .get_auxiliary_details()
                .await
                .context("failed to retrieve the node's announced ports")?
                .announce_ports
                .mix_port
                .unwrap_or(DEFAULT_MIX_LISTENING_PORT),
        );

        let roles = api_client
            .get_roles()
            .await
            .context("failed to retrieve the node's roles")?;

        // asked for only of a gateway-capable node, since a pure mixnode serves no client websocket
        let clients_ws_port = if roles.gateway_enabled {
            Some(
                api_client
                    .get_mixnet_websockets()
                    .await
                    .context("failed to retrieve the node's client websocket interface")?
                    .ws_port,
            )
        } else {
            None
        };

        let discovered = DiscoveredNode {
            mixnet: TestedNodeDetails {
                // a manually targeted node has no contract id, and nothing on this path submits a
                // result that would need one
                node_id: None,
                address,
                known_ips,
                noise_key,
                key_rotation: SphinxKeyRotation::from_key_rotation_id(
                    host_info.keys.primary_x25519_sphinx_key.rotation_id,
                ),
                sphinx_key: host_info.keys.primary_x25519_sphinx_key.public_key,
            },
            identity: host_info.keys.ed25519_identity,
            roles,
            clients_ws_port,
        };

        info!(
            "{host} is {} at {}, mixnode: {}, gateway: {}",
            discovered.identity,
            discovered.mixnet.address,
            roles.mixnode_enabled,
            roles.gateway_enabled
        );

        Ok(discovered)
    }

    /// The node as a mixnode probe's target, refusing one that does not mix.
    ///
    /// Refused up front rather than probed anyway: a node with mixing disabled drops forward-hop
    /// packets outright, so the run would score zero and look like a delivery failure instead of a
    /// command aimed at the wrong node.
    pub(crate) fn require_mixnode(self) -> anyhow::Result<TestedNodeDetails> {
        if !self.roles.mixnode_enabled {
            bail!(
                "{} does not operate as a mixnode, so it will not forward the probe's packets",
                self.identity
            )
        }
        Ok(self.mixnet)
    }

    /// The node as a gateway probe's target, refusing one that is not an entry gateway.
    pub(crate) fn require_gateway(self) -> anyhow::Result<TestedGatewayDetails> {
        if !self.roles.gateway_enabled {
            bail!(
                "{} does not operate as an entry gateway, so it serves no client websocket to probe",
                self.identity
            )
        }

        // present for every gateway-capable node, since that is exactly when it is queried above
        let clients_ws_port = self
            .clients_ws_port
            .context("the node reports the gateway role but announced no client websocket port")?;

        Ok(TestedGatewayDetails {
            mixnet: self.mixnet,
            identity: self.identity,
            clients_ws_port,
        })
    }
}
