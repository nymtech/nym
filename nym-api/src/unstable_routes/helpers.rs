// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::described::v3::{NymNodeDescriptionConversion, NymNodeDescriptionV3};
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::models::{NodeAnnotationV1, NodeAnnotationV2};
use nym_api_requests::nym_nodes::NodeRole;
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use time::OffsetDateTime;

pub(crate) fn refreshed_at(
    iter: impl IntoIterator<Item = OffsetDateTime>,
) -> OffsetDateTimeJsonSchemaWrapper {
    iter.into_iter()
        .min()
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .into()
}

/// Given all relevant caches, build part of response for JUST Nym Nodes
pub(crate) fn build_nym_nodes_response<'a, NI, O>(
    rewarded_set: &CachedEpochRewardedSet,
    nym_nodes_subset: NI,
    annotations: &HashMap<NodeId, NodeAnnotationV2>,
    current_key_rotation: u32,
    active_only: bool,
) -> Vec<O>
where
    NI: Iterator<Item = &'a NymNodeDescriptionV3> + 'a,
    O: NymNodeDescriptionConversion,
{
    let mut nodes = Vec::new();
    for nym_node in nym_nodes_subset {
        let node_id = nym_node.node_id;

        let role: NodeRole = rewarded_set.role(node_id).into();

        // if the role is inactive, see if our filter allows it
        if active_only && role.is_inactive() {
            continue;
        }

        // honestly, not sure under what exact circumstances this value could be missing,
        // but in that case just use 0 performance
        let annotation: NodeAnnotationV1 = annotations
            .get(&node_id)
            .cloned()
            .unwrap_or_default()
            .into();

        if let Some(node) = O::from_description(
            nym_node,
            current_key_rotation,
            role,
            annotation.last_24h_performance,
        ) {
            nodes.push(node);
        };
    }
    nodes
}
