// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Topology file types and the in-memory network directory.
//!
//! The topology is loaded from `topology.json` and contains everything needed
//! to construct a node or client (including private config such as keys).
//! The [`directory::Directory`] holds only the public-facing routing information
//! visible to other participants in the network.

use std::net::SocketAddr;

use nym_crypto::asymmetric::x25519;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_private_key;
use serde::{Deserialize, Serialize};

use crate::{client::ClientId, node::NodeId};

pub mod directory;

/// Per-node configuration stored in `topology.json`.
#[derive(Serialize, Deserialize)]
pub struct TopologyNode {
    /// Unique identifier for this node within the topology.
    pub node_id: NodeId,
    /// UDP address on which the node listens for incoming packets.
    pub socket_address: SocketAddr,
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
    pub fn new(node_id: NodeId, reliability: u8, socket_address: SocketAddr) -> Self {
        let sphinx_private_key = x25519::PrivateKey::new(&mut rand::thread_rng());
        Self {
            node_id,
            socket_address,
            reliability,
            sphinx_private_key,
        }
    }
}

/// Per-client configuration stored in `topology.json`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyClient {
    /// Unique identifier for this client within the topology.
    pub client_id: ClientId,
    /// UDP address the client uses to talk to the mix network.
    pub mixnet_address: SocketAddr,
    /// UDP address where the client listens for messages from user applications
    /// (e.g. the standalone `client` binary).  Not included in the
    /// [`Directory`](directory::Directory).
    pub app_address: SocketAddr,
}

impl TopologyClient {
    /// Construct a [`TopologyClient`] with the given addresses.
    ///
    /// Intended for use by `init-topology` to generate a topology file for the
    /// simulation.
    pub fn new(client_id: ClientId, mixnet_address: SocketAddr, app_address: SocketAddr) -> Self {
        Self {
            client_id,
            mixnet_address,
            app_address,
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
