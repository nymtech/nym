// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Topology file types and the in-memory network directory.
//!
//! The topology is loaded from `topology.json` and contains everything needed
//! to construct a node or client (including private config such as keys).
//! The [`directory::Directory`] holds only the public-facing routing information
//! visible to other participants in the network.

use std::net::SocketAddr;

use anyhow::{Context, bail};
use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_private_key;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, EnumIter, IntoEnumIterator};

use crate::{client::ClientId, node::NodeId};

pub mod directory;

/// What a node does for the length of the run, as an epoch's rewarded set would say.
///
/// Roles are disjoint, and that is the point: a route runs entry gateway, the three mix layers,
/// then the recipient's gateway, so consecutive hops are always drawn from different roles. A node
/// is therefore never its own next hop - which LP could not carry anyway, since the two halves of a
/// session share a receiver index but not a direction.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, EnumIter, EnumCount)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    Layer1,
    Layer2,
    Layer3,
    /// Entry and exit at once: clients send through one and are reached through one.
    Gateway,
}

/// Per-node configuration stored in `topology.json`.
#[derive(Serialize, Deserialize)]
pub struct TopologyNode {
    /// Unique identifier for this node within the topology.
    pub node_id: NodeId,
    /// UDP address on which the node listens for incoming packets.
    pub socket_address: SocketAddr,
    /// What this node does in the network - see [`NodeRole`].
    pub role: NodeRole,
    /// Notional reliability percentage (0–100); reserved for future use.
    pub reliability: u8,
    /// Sphinx (X25519) private key used by this node to unwrap packets.
    #[serde(with = "bs58_x25519_private_key")]
    pub sphinx_private_key: x25519::PrivateKey,
}

impl TopologyNode {
    /// Construct a [`TopologyNode`] with a freshly generated Sphinx keypair.
    ///
    /// Intended for use by `init-topology` to generate a topology file for the
    /// simulation.
    pub fn new(
        node_id: NodeId,
        reliability: u8,
        socket_address: SocketAddr,
        role: NodeRole,
    ) -> Self {
        let sphinx_private_key = x25519::PrivateKey::new(&mut rand::thread_rng());
        Self {
            node_id,
            socket_address,
            role,
            reliability,
            sphinx_private_key,
        }
    }
}

/// Per-client configuration stored in `topology.json`.
#[derive(Serialize, Deserialize)]
pub struct TopologyClient {
    /// Unique identifier for this client within the topology.
    pub client_id: ClientId,
    /// UDP address the client uses to talk to the mix network.
    pub mixnet_address: SocketAddr,
    /// UDP address where the client listens for messages from user applications
    /// (e.g. the standalone `client` binary).  Not included in the
    /// [`Directory`](directory::Directory).
    pub app_address: SocketAddr,
    /// Sphinx (X25519) private key used by this client to unwrap packets.
    #[serde(with = "bs58_x25519_private_key")]
    pub sphinx_private_key: x25519::PrivateKey,
}

impl TopologyClient {
    /// Construct a [`TopologyClient`] with the given addresses.
    ///
    /// Intended for use by `init-topology` to generate a topology file for the
    /// simulation.
    pub fn new(client_id: ClientId, mixnet_address: SocketAddr, app_address: SocketAddr) -> Self {
        let sphinx_private_key = x25519::PrivateKey::new(&mut rand::thread_rng());
        Self {
            client_id,
            mixnet_address,
            app_address,
            sphinx_private_key,
        }
    }
}

/// Root topology file structure, deserialised from `topology.json`.
#[derive(Serialize, Deserialize)]
pub struct Topology {
    /// Every mix node participating in the simulation.
    pub nodes: Vec<TopologyNode>,
    /// Every simulated client with sockets bound to localhost.
    pub clients: Vec<TopologyClient>,
}

impl Topology {
    /// Read and validate a topology file.
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let data = std::fs::read_to_string(path).context("Failed to read topology file")?;
        let topology: Self = serde_json::from_str(&data).context("Topology file malformed")?;

        topology.validate()?;

        Ok(topology)
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.nodes.len() < NodeRole::COUNT {
            bail!(
                "a simulation needs at least {} nodes, this topology has {}",
                NodeRole::COUNT,
                self.nodes.len()
            );
        }

        // a route touches every role in turn, so one unfilled role means no route exists
        for role in NodeRole::iter() {
            if !self.nodes.iter().any(|node| node.role == role) {
                bail!("no node has the {role:?} role, so no route can be built");
            }
        }

        Ok(())
    }
}
