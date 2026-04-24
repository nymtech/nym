// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::topology::directory::NodeId;

pub mod directory;

// Topology is from the file, has everything needed to build a node
// Directory is the directory of public information for nodes, what can be seen to everybody

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
