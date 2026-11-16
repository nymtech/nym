// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_nodes::node_view::NodeSubset;
use crate::support::attested_snapshots::data::RetainedHeight;
use nym_api_requests::models::described::v3::NymNodeDescriptionConversion;
use nym_api_requests::nym_nodes::NodeRole;

/// The nodes of `subset` listed at the snapshot's height, with their roles, keys and performance
/// at that height.
pub(crate) fn build_nym_nodes_response<O>(
    snapshot: &RetainedHeight,
    subset: NodeSubset,
    active_only: bool,
) -> Vec<O>
where
    O: NymNodeDescriptionConversion,
{
    let rewarded_set = &snapshot.mixnet().rewarded_set;
    let current_key_rotation = snapshot.mixnet().current_key_rotation_id();

    let mut nodes = Vec::new();
    for nym_node in snapshot.node_view.nodes(subset) {
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
