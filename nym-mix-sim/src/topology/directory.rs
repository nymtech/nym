// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! In-memory network directory used by nodes to resolve [`NodeId`]s to socket
//! addresses at send time.
//!
//! The [`Directory`] is built once during driver initialisation (after all UDP
//! sockets have been bound) and then shared immutably across every node via an
//! [`Arc`](std::sync::Arc). This means routing lookups are lock-free and
//! allocation-free after startup.

use std::{collections::HashMap, net::SocketAddr};

use nym_crypto::asymmetric::{ed25519, x25519};
use nym_sphinx::{Destination, DestinationAddressBytes, Node as SphinxNode};
use nym_sphinx_addressing::{ClientAddress, nodes::NymNodeRoutingAddress};
use nym_topology::{NymTopology, RoutingNode, SupportedRoles};
use rand::{SeedableRng, rngs::StdRng, seq::IteratorRandom};

use crate::{
    client::ClientId,
    node::NodeId,
    topology::{Topology, TopologyClient, TopologyNode},
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
    clients: HashMap<ClientId, DirectoryClient>,
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
    pub fn client(&self, id: ClientId) -> Option<&DirectoryClient> {
        self.clients.get(&id)
    }

    /// Pick a random node from the directory and return its [`NodeId`].
    ///
    /// Used by Sphinx clients to choose a first-hop node when the simulation has
    /// no explicit gateway concept.
    pub fn random_next_hop(&self, rng: &mut impl rand::Rng) -> DirectoryNode {
        // SAFETY: The directory always contains at least one node in a valid simulation.
        #[allow(clippy::unwrap_used)]
        *self.nodes.values().choose(rng).unwrap()
    }

    /// Sample `length` random hops for a Sphinx route, never twice in a row.
    ///
    /// A node may still appear more than once in the route, just not as its own next hop. A real
    /// mixnet draws each hop from a disjoint layer, so it never sends a packet to itself; allowing
    /// it here would ask the LP transport to encrypt and decrypt on one session, which the two
    /// halves of a handshake cannot do - they share a receiver index but not a direction.
    /// If `first_hop` is provided, it is used as the `previous` node for the first hop, to disallow repeating node
    pub fn random_route(
        &self,
        length: usize,
        rng: &mut impl rand::Rng,
        first_hop: Option<SocketAddr>,
    ) -> Vec<DirectoryNode> {
        let mut route: Vec<DirectoryNode> = Vec::with_capacity(length);

        while route.len() < length {
            let previous = route.last().map(|node| node.addr).or(first_hop);

            // SAFETY: a validated topology holds at least `MIN_NODES` nodes, so excluding the
            // previous hop always leaves one to choose from.
            #[allow(clippy::unwrap_used)]
            let hop = *self
                .nodes
                .values()
                .filter(|node| previous != Some(node.addr))
                .choose(rng)
                .unwrap();

            route.push(hop);
        }

        route
    }

    /// Build a [`NymTopology`] view of the directory for the real nym-node data
    /// pipeline's gateway state.
    pub fn as_nym_topology(&self) -> NymTopology {
        let mut topology = NymTopology::default();
        for node in self.nodes.values() {
            topology.insert_node_details(node.as_routing_node());
        }
        topology
    }

    pub fn as_client_map(&self) -> HashMap<ClientAddress, SocketAddr> {
        let mut map = HashMap::new();
        for client in self.clients.values() {
            map.insert(client.client_address(), client.addr);
        }
        map
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
            directory.clients.insert(client.client_id, client.into());
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

impl DirectoryNode {
    pub fn as_sphinx_node_socket(&self) -> SphinxNode {
        let address = NymNodeRoutingAddress::Node(self.addr);
        // SAFETY : our addressing scheme can fit in a sphinx packet
        #[allow(clippy::unwrap_used)]
        SphinxNode::new(address.try_into().unwrap(), *self.sphinx_public_key)
    }

    /// Derive the [`RoutingNode`] entry used by the real nym-node gateway state.
    /// The id key is unused
    fn as_routing_node(&self) -> RoutingNode {
        let mut rng = StdRng::seed_from_u64(self.id as u64);
        let identity_key = ed25519::PrivateKey::new(&mut rng).public_key();
        RoutingNode {
            node_id: self.id as u32,
            mix_host: self.addr,
            entry: None,
            identity_key,
            sphinx_key: self.sphinx_public_key,
            supported_roles: SupportedRoles {
                mixnode: true,
                mixnet_entry: true,
                mixnet_exit: true,
            },
            // the simulator wires LP peers up directly from `topology.json` rather than through
            // anything directory-shaped
            lp: None,
            build_version: None,
        }
    }
}

/// Public routing information for a client, stored in the [`Directory`].
#[derive(Copy, Clone, Debug)]
pub struct DirectoryClient {
    /// Unique identifier for this client within the topology.
    ///
    /// Used as the key in the [`Directory`] when resolving routing targets.
    pub id: ClientId,

    /// UDP socket address on which this client listens for incoming packets.
    pub addr: SocketAddr,

    /// Sphinx (X25519) public key used to encrypt packets destined for this client.
    pub sphinx_public_key: x25519::PublicKey,
}

impl From<&TopologyClient> for DirectoryClient {
    /// Derive the public [`DirectoryClient`] entry from a [`TopologyClient`] by
    /// computing the corresponding X25519 public key from the private key.
    fn from(value: &TopologyClient) -> Self {
        DirectoryClient {
            id: value.client_id,
            addr: value.mixnet_address,
            sphinx_public_key: x25519::PublicKey::from(&value.sphinx_private_key),
        }
    }
}

impl DirectoryClient {
    pub fn as_sphinx_node(&self) -> SphinxNode {
        // For the simulation, just repeat the id in lieu of client address
        let address = NymNodeRoutingAddress::Client(self.client_address());
        // SAFETY : our addressing scheme can fit in a sphinx packet
        #[allow(clippy::unwrap_used)]
        SphinxNode::new(address.try_into().unwrap(), *self.sphinx_public_key)
    }

    pub fn client_address(&self) -> ClientAddress {
        ClientAddress::from_bytes([self.id; 20])
    }

    pub fn as_sphinx_destination(&self) -> Destination {
        // For the simulation, just repeat the ID
        Destination::new(
            DestinationAddressBytes::from_bytes([self.id; 32]),
            [self.id; 16],
        )
    }
}
