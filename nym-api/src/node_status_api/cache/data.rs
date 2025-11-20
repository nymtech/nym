// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::NodeAnnotation;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;

#[derive(Default)]
#[allow(deprecated)]
pub(crate) struct NodeStatusCacheData {
    /// Basic annotation for nym-nodes
    pub(crate) node_annotations: HashMap<NodeId, NodeAnnotation>,
}

impl From<HashMap<NodeId, NodeAnnotation>> for NodeStatusCacheData {
    fn from(node_annotations: HashMap<NodeId, NodeAnnotation>) -> Self {
        NodeStatusCacheData { node_annotations }
    }
}
