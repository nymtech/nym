// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::{GatewayBondAnnotated, MixNodeBondAnnotated, NodeAnnotation};
use nym_contracts_common::IdentityKey;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;

use crate::support::caching::Cache;

use super::inclusion_probabilities::InclusionProbabilities;

#[derive(Default)]
pub(crate) struct NodeStatusCacheData {
    pub(crate) legacy_gateway_mapping: Cache<HashMap<IdentityKey, NodeId>>,

    /// Basic annotation for **all** nodes, i.e. legacy + nym-nodes
    pub(crate) node_annotations: Cache<HashMap<NodeId, NodeAnnotation>>,

    /// Annotations as before, just for legacy things
    pub(crate) mixnodes_annotated: Cache<HashMap<NodeId, MixNodeBondAnnotated>>,
    pub(crate) gateways_annotated: Cache<HashMap<NodeId, GatewayBondAnnotated>>,

    // Estimated active set inclusion probabilities from Monte Carlo simulation
    pub(crate) inclusion_probabilities: Cache<InclusionProbabilities>,
}

impl NodeStatusCacheData {
    pub fn new() -> Self {
        Self::default()
    }
}
