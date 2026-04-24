// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! In-memory network directory used by nodes to resolve [`NodeId`]s to socket
//! addresses at send time.
//!
//! The [`Directory`] is built once during driver initialisation (after all UDP
//! sockets have been bound) and then shared immutably across every [`Node`] via
//! an [`Arc`]. This means routing lookups are lock-free and allocation-free after
//! startup.

use std::{collections::HashMap, net::SocketAddr};

use crate::{
    client::{Client, ClientId},
    node::{Node, NodeId},
};

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
    nodes: HashMap<NodeId, DirectoryNode>,
    clients: HashMap<ClientId, SocketAddr>,
}

impl Directory {
    /// Construct a [`Directory`] from a fully-initialised slice of [`Node`]s.
    ///
    /// Iterates over `node_list`, calls [`Node::directory_node`] on each
    /// entry, and inserts the result keyed by [`DirectoryNode::id`].
    ///
    /// Overwrites earlier entries if two nodes share the same ID — callers
    /// should ensure IDs are unique.
    pub fn build_from_nodes<Ts, Fr, Pkt>(
        node_list: &Vec<Node<Ts, Pkt>>,
        client_list: &Vec<Client<Ts, Fr, Pkt>>,
    ) -> Self {
        let mut nodes = HashMap::new();
        for node in node_list {
            let directory_node = node.directory_node();
            nodes.insert(directory_node.id, directory_node);
        }
        let mut clients = HashMap::new();
        for client in client_list {
            clients.insert(client.id(), client.mixnet_address());
        }
        Self { nodes, clients }
    }

    /// Look up a node by its [`NodeId`].
    ///
    /// Returns `None` when `id` is not present in the directory
    pub fn node(&self, id: NodeId) -> Option<&DirectoryNode> {
        self.nodes.get(&id)
    }

    /// Look up a client by its [`ClientId`].
    ///
    /// Returns `None` when `id` is not present in the directory
    pub fn client(&self, id: NodeId) -> Option<&SocketAddr> {
        self.clients.get(&id)
    }
}

/// Public routing information for a single mix node, stored in the [`Directory`].
///
#[derive(Clone, Debug)]
pub struct DirectoryNode {
    /// Unique identifier for this node within the topology.
    ///
    /// Used as the key in the [`Directory`] when resolving routing targets.
    pub id: NodeId,

    /// UDP socket address on which this node listens for incoming packets.
    pub addr: SocketAddr,
}
