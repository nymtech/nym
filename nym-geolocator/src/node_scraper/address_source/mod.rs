// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nyx::nodes::MinimalNymNode;
use async_trait::async_trait;
use nym_validator_client::client::NodeId;
use std::collections::HashMap;
use std::net::IpAddr;

pub(crate) mod http;

/// The addresses a single node is to be geolocated at.
#[derive(Clone)]
pub(crate) struct NodeAddresses {
    pub(crate) node_id: NodeId,
    pub(crate) addresses: Vec<IpAddr>,
}

/// Where the geolocator learns the addresses of the nodes it measures.
///
/// Exists so the directory contract can replace the node's own http endpoint without the
/// measurement, batching or submission paths changing. That source publishes the same two fields
/// this one scrapes (`NodeInformation::hostname` and `NodeInformation::ip_addresses`) but as a
/// single node-signed contract query rather than one http round trip per node, so everything
/// downstream consumes [`NodeAddresses`], which deliberately carries no trace of the origin.
#[async_trait]
pub(crate) trait AddressSource: Send + Sync {
    /// Discover the addresses of every given node, omitting the ones that could not be resolved.
    ///
    /// Takes the whole set rather than one node at a time because a source is free to answer for
    /// all of them at once - the directory contract is a paged query, not a query per node - and a
    /// per-node method would force such a source into either N round trips or a cache of its own.
    async fn discover(&self, nodes: &HashMap<NodeId, MinimalNymNode>) -> Vec<NodeAddresses>;
}
