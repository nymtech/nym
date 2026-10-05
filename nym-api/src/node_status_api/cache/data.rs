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

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use cosmwasm_std::coin;
    use nym_api_requests::models::{
        ChainInteractionCapabilitiesDetailed, DetailedNodePerformanceV2, DisplayRole,
    };

    /// Every `Option` is `Some`, since a `None` never encodes the type it wraps and so
    /// would leave that branch of the encoder untested.
    fn annotation(role: DisplayRole) -> NodeAnnotationV2 {
        NodeAnnotationV2 {
            current_role: Some(role),
            chain_interaction_capabilities: Some(ChainInteractionCapabilitiesDetailed {
                on_chain_balance: coin(1_000_000, "unym"),
                is_feegrant_grantee: true,
            }),
            detailed_performance: DetailedNodePerformanceV2::default(),
        }
    }

    // This cache is persisted to disk with bincode; a populated value must survive that
    // round trip or the on-disk cache silently never writes.
    #[test]
    fn populated_cache_round_trips_through_the_on_disk_format() -> anyhow::Result<()> {
        let data = NodeStatusCacheData {
            node_annotations: HashMap::from([
                (1, annotation(DisplayRole::EntryGateway)),
                (2, annotation(DisplayRole::Layer1)),
            ]),
        };

        let restored = round_trip_through_disk_cache(data)?;

        assert_eq!(restored.node_annotations.len(), 2);
        let first = &restored.node_annotations[&1];
        assert!(first.chain_interaction_capabilities.is_some());
        assert!(matches!(
            first.current_role,
            Some(DisplayRole::EntryGateway)
        ));
        Ok(())
    }
}
