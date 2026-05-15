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
use nym_sphinx::{Node as SphinxNode, NodeAddressBytes};
use rand::{prelude::SliceRandom, seq::IteratorRandom};

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
    /// Mix-network socket address for each client, keyed by [`ClientId`].
    ///
    /// Used by nodes to deliver final-hop packets directly to the target client's
    /// mix socket rather than forwarding to another node.
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

    /// Returns the node_id of every node in the network
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }

    /// Pick a random node
    pub fn random_next_hop(&self, rng: &mut impl rand::Rng) -> NodeId {
        // SAFETY: The directory always contains at least one node in a valid simulation.
        #[allow(clippy::unwrap_used)]
        *self.node_ids().choose(rng).unwrap()
    }

    pub fn random_route(&self, length: usize, rng: &mut impl rand::Rng) -> Vec<DirectoryNode> {
        // SAFETY: The directory always contains at least one node in a valid simulation.
        #[allow(clippy::unwrap_used)]
        std::iter::repeat_with(|| *self.nodes.values().choose(rng).unwrap())
            .take(length)
            .collect()
    }
}

impl From<&Topology> for Directory {
    /// Build a [`Directory`] from a full [`Topology`], extracting only the
    /// public routing information (addresses and public keys) from each entry.
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
#[derive(Copy, Clone, Debug)]
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
    /// Derive the public [`DirectoryNode`] entry from a [`TopologyNode`] by
    /// computing the corresponding X25519 public key from the private key.
    fn from(value: &TopologyNode) -> Self {
        DirectoryNode {
            id: value.node_id,
            addr: value.socket_address,
            sphinx_public_key: x25519::PublicKey::from(&value.sphinx_private_key),
        }
    }
}

impl From<&DirectoryNode> for SphinxNode {
    /// Convert a [`DirectoryNode`] into a [`SphinxNode`] suitable for use with
    /// [`SphinxPacketBuilder`].
    ///
    /// The Sphinx [`NodeAddressBytes`] are constructed by repeating the single-byte
    /// [`NodeId`] across all 32 bytes — a simulation-only convention that lets the
    /// node recover its own ID from the address after decryption.
    ///
    /// [`SphinxPacketBuilder`]: nym_sphinx::SphinxPacketBuilder
    fn from(value: &DirectoryNode) -> Self {
        let address = NodeAddressBytes::from_bytes([value.id; 32]);
        SphinxNode::new(address, *value.sphinx_public_key)
    }
}

impl From<DirectoryNode> for SphinxNode {
    fn from(value: DirectoryNode) -> Self {
        (&value).into()
    }
}
