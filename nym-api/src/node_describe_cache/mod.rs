// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::cache::UninitialisedCache;
use nym_api_requests::models::NymNodeDescription;
use nym_config::defaults::DEFAULT_NYM_NODE_HTTP_PORT;
use nym_mixnet_contract_common::NodeId;
use nym_node_requests::api::client::NymNodeApiClientError;
use nym_topology::node::RoutingNodeError;
use nym_topology::RoutingNode;
use thiserror::Error;

pub(crate) mod cache;
pub(crate) mod provider;
mod query_helpers;
pub(crate) mod refresh;

#[derive(Debug, Error)]
pub enum NodeDescribeCacheError {
    #[error("contract cache hasn't been initialised")]
    UninitialisedContractCache {
        #[from]
        source: UninitialisedCache,
    },

    #[error("node {node_id} has provided malformed host information ({host}: {source}")]
    MalformedHost {
        host: String,

        node_id: NodeId,

        #[source]
        source: NymNodeApiClientError,
    },

    #[error("node {node_id} with host '{host}' doesn't seem to expose its declared http port nor any of the standard API ports, i.e.: 80, 443 or {}", DEFAULT_NYM_NODE_HTTP_PORT)]
    NoHttpPortsAvailable { host: String, node_id: NodeId },

    #[error("failed to query node {node_id}: {source}")]
    ApiFailure {
        node_id: NodeId,

        #[source]
        source: NymNodeApiClientError,
    },

    // TODO: perhaps include more details here like whether key/signature/payload was malformed
    #[error("could not verify signed host information for node {node_id}")]
    MissignedHostInformation { node_id: NodeId },

    #[error("identity of node {node_id} does not match. expected {expected} but got {got}")]
    MismatchedIdentity {
        node_id: NodeId,
        expected: String,
        got: String,
    },

    #[error("node {node_id} is announcing an illegal ip address")]
    IllegalIpAddress { node_id: NodeId },
}

// this exists because I've been moving things around quite a lot and now the place that holds the type
// doesn't have relevant dependencies for proper impl
pub(crate) trait NodeDescriptionTopologyExt {
    fn try_to_topology_node(
        &self,
        current_rotation_id: u32,
    ) -> Result<RoutingNode, RoutingNodeError>;
}

impl NodeDescriptionTopologyExt for NymNodeDescription {
    fn try_to_topology_node(
        &self,
        current_rotation_id: u32,
    ) -> Result<RoutingNode, RoutingNodeError> {
        // for the purposes of routing, performance is completely ignored,
        // so add dummy value and piggyback on existing conversion
        (&self.to_skimmed_node(current_rotation_id, Default::default(), Default::default()))
            .try_into()
    }
}
