// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::Cache;
use nym_api_requests::models::NodeAnnotation;
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;

#[derive(Default)]
#[allow(deprecated)]
pub(crate) struct NodeStatusCacheData {
    /// Basic annotation for nym-nodes
    pub(crate) node_annotations: Cache<HashMap<NodeId, NodeAnnotation>>,
}

impl NodeStatusCacheData {
    pub fn new() -> Self {
        Self::default()
    }
}
