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

use nym_crypto::asymmetric::x25519;

use crate::{
    client::ClientId,
    node::NodeId,
    topology::{Topology, TopologyNode},
};

/// Shared, immutable routing table for the simulation.
///
/// Maps every [`NodeId`] that is part of the current topology to a
/// [`DirectoryNode`] entry containing the node's configuration and reachable
/// [`SocketAddr`].
#[derive(Default, Debug)]
pub struct Directory {
    /// Keyed routing map: node ID → directory entry.
    nodes: HashMap<NodeId, DirectoryNode>,
    clients: HashMap<ClientId, SocketAddr>,
}

impl Directory {
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

impl From<&Topology> for Directory {
    fn from(value: &Topology) -> Self {
        let mut directory = Directory::default();
        for node in &value.nodes {
            directory.nodes.insert(node.node_id, node.into());
        }
        for client in &value.clients {
            directory
                .clients
                .insert(client.client_id, client.mixnet_address);
        }
        directory
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

    /// Sphinx (X25519) public key used to encrypt packets destined for this node.
    pub sphinx_public_key: x25519::PublicKey,
}

impl From<&TopologyNode> for DirectoryNode {
    fn from(value: &TopologyNode) -> Self {
        DirectoryNode {
            id: value.node_id,
            addr: value.socket_address,
            sphinx_public_key: x25519::PublicKey::from(&value.sphinx_private_key),
        }
    }
}
