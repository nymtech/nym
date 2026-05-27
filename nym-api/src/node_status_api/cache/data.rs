// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::NodeAnnotationV2;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Serialize, Deserialize)]
#[allow(deprecated)]
pub(crate) struct NodeStatusCacheData {
    /// Basic annotation for nym-nodes
    pub(crate) node_annotations: HashMap<NodeId, NodeAnnotationV2>,
}

impl From<HashMap<NodeId, NodeAnnotationV2>> for NodeStatusCacheData {
    fn from(node_annotations: HashMap<NodeId, NodeAnnotationV2>) -> Self {
        NodeStatusCacheData { node_annotations }
    }
}
