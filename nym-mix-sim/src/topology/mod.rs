// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Topology file types and the in-memory network directory.
//!
//! The topology is loaded from `topology.json` and contains everything needed
//! to construct a node or client (including private config such as keys).
//! The [`directory::Directory`] holds only the public-facing routing information
//! visible to other participants in the network.

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::{client::ClientId, node::NodeId};

pub mod directory;

/// Per-node configuration stored in `topology.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyNode {
    pub node_id: NodeId,
    /// UDP address on which the node listens for incoming packets.
    pub socket_address: SocketAddr,
    /// Notional reliability percentage (0–100); reserved for future use.
    pub reliability: u8,
    //sphinx_private_key: String,
    //sphinx_public_key: String,
}

impl TopologyNode {
    /// Construct a [`TopologyNode`].
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
