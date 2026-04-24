// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! In-memory network directory used by nodes to resolve [`NodeId`]s to socket
//! addresses at send time.
//!
//! The [`Directory`] is built once during driver initialisation (after all UDP
//! sockets have been bound) and then shared immutably across every [`Node`] via
//! an [`Arc`]. This means routing lookups are lock-free and allocation-free after
//! startup.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

use crate::node::Node;

/// Shared, immutable routing table for the simulation.
///
/// Maps every [`NodeId`] that is part of the current topology to a
/// [`DirectoryNode`] entry containing the node's configuration and reachable
/// [`SocketAddr`].
///
/// Built once via [`Directory::build_from_nodes`] and then distributed to all
/// [`Node`]s as an [`Arc<Directory>`] so that every node can resolve
/// destinations without holding a mutable reference to the driver.
#[derive(Default, Debug)]
pub struct Directory {
    /// Keyed routing map: node ID → directory entry.
    nodes: HashMap<NodeId, TopologyNode>,
}

impl Directory {
    /// Construct a [`Directory`] from a fully-initialised slice of [`Node`]s.
    ///
    /// Iterates over `node_list`, calls [`Node::get_directory_node`] on each
    /// entry, and inserts the result keyed by [`Node::id`].
    //
    ///
    /// Overwrites earlier entries if two nodes
    /// share the same ID — callers should ensure IDs are unique.
    pub fn build_from_nodes<Ts, Pkt, Fr, Pl>(node_list: &Vec<Node<Ts, Pkt, Fr, Pl>>) -> Self {
        let mut nodes = HashMap::new();
        for node in node_list {
            nodes.insert(node.id(), node.get_topology_node());
        }
        Self { nodes }
    }

    /// Look up a node by its [`NodeId`].
    ///
    /// Returns `None` when `id` is not present in the directory
    pub fn node(&self, id: NodeId) -> Option<&TopologyNode> {
        self.nodes.get(&id)
    }
}

/// Compact identifier for a mix node in the simulation topology.
///
/// `u8` keeps the IDs small (max 255 nodes) and is large enough for any
/// realistic simulated topology.
pub type NodeId = u8;

/// Serialisable description of a mix node, used as the interchange format in
/// `topology.json`.
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopologyNode {
    /// Unique identifier for this node within the topology.
    ///
    /// Used as the key in the [`Directory`] when resolving routing targets.
    pub id: NodeId,

    /// Simulated packet-delivery reliability expressed as a percentage
    /// Used by the simulation engine to decide
    /// whether to drop a given packet, modelling real-world link unreliability.
    pub reliability: u8,

    /// UDP socket address on which this node listens for incoming packets.
    pub addr: SocketAddr,
}

impl TopologyNode {
    /// Construct a new [`TopologyNode`] with the given parameters.
    pub fn new(id: NodeId, reliability: u8, addr: SocketAddr) -> Self {
        Self {
            id,
            reliability,
            addr,
        }
    }
}
