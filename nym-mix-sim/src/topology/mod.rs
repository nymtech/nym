// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::topology::directory::NodeId;

pub mod directory;

// Topology is loaded from file and contains everything needed to construct a
// node or client (including private config).
// Directory holds only the public-facing node information visible to other
// participants in the network.

/// Compact identifier for a simulated client.
pub type ClientId = u8;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyNode {
    pub node_id: NodeId,
    pub socket_address: SocketAddr,
    pub reliability: u8,
    //sphinx_private_key: String,
    //sphinx_public_key: String,
}

impl TopologyNode {
    /// Construct a [`TopologyNode`] with placeholder key strings.
    ///
    /// Intended for use by `init-topology` to generate a topology file for the
    /// simulation.
    pub fn new(node_id: NodeId, reliability: u8, socket_address: SocketAddr) -> Self {
        Self {
            node_id,
            socket_address,
            reliability,
            //sphinx_private_key: format!("placeholder_private_key_{node_id}"),
            //sphinx_public_key: format!("placeholder_public_key_{node_id}"),
        }
    }
}

/// Per-client configuration stored in `topology.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyClient {
    pub client_id: ClientId,
    /// UDP address the client uses to talk to the mix network.
    pub mixnet_address: SocketAddr,
    /// UDP address where the client listens for messages from user applications
    /// (e.g. the standalone `client` binary).  Not included in the [`Directory`].
    pub app_address: SocketAddr,
}

impl TopologyClient {
    pub fn new(client_id: ClientId, mixnet_address: SocketAddr, app_address: SocketAddr) -> Self {
        Self {
            client_id,
            mixnet_address,
            app_address,
        }
    }
}

/// Root topology file structure.
///
/// Replaces the earlier bare `Vec<TopologyNode>` so that clients can live
/// alongside nodes in the same file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Topology {
    pub nodes: Vec<TopologyNode>,
    pub clients: Vec<TopologyClient>,
}
