// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::attested_snapshots::data::RetainedHeight;
use nym_api_requests::models::described::v3::{NymNodeDescriptionConversion, NymNodeDescriptionV3};
use nym_api_requests::models::OffsetDateTimeJsonSchemaWrapper;
use nym_api_requests::nym_nodes::NodeRole;
use nym_topology::CachedEpochRewardedSet;
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
    snapshot: &RetainedHeight,
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

        if let Some(node) = O::from_description(
            nym_node,
            current_key_rotation,
            role,
            snapshot.published_performance(node_id),
        ) {
            nodes.push(node);
        };
    }
    nodes
}
