// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The routing view: every node bonded at the verified height, with its newest committed
//! score and the epoch that score came from.

use crate::verify::VerifiedPerformance;
use nym_performance_contract_common::{LatestPerformance, NodeId};
use nym_validator_client::nyxd::Height;
use std::collections::BTreeMap;

/// Every node bonded at `height`, with its committed entry, or `None` when it has never been
/// measured.
///
/// A node's committed entry is its bundle at the current epoch when it was measured in it,
/// and its newest earlier bundle otherwise, so this is "the score now, else the last known
/// one". No entry is dropped, discounted or flagged for its age: each carries its source
/// epoch, so a consumer that wants a bound applies its own.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingView {
    /// The height the view was verified at, for joining it with other reads at that height.
    pub height: Height,

    pub nodes: BTreeMap<NodeId, Option<LatestPerformance>>,
}

impl VerifiedPerformance {
    /// The routing view of this verified set: one entry per bonded node. A committed entry
    /// whose node is not bonded at the verified height is left out of the view, and stays in
    /// [`Self::entries`].
    pub fn routing_view(&self) -> RoutingView {
        RoutingView {
            height: self.height,
            nodes: self
                .node_identities
                .keys()
                .map(|node_id| (*node_id, self.entries.get(node_id).cloned()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{bonded, entry};
    use nym_lthash::LtHash16;

    /// A verified set holding `entries` with `bonded_nodes` bonded. The accumulator plays no
    /// part in the view.
    fn verified(entries: Vec<LatestPerformance>, bonded_nodes: &[NodeId]) -> VerifiedPerformance {
        VerifiedPerformance {
            height: Height::from(100u32),
            accumulator: LtHash16::new(),
            entries: entries
                .into_iter()
                .map(|entry| (entry.node_id, entry))
                .collect(),
            node_identities: bonded(bonded_nodes),
        }
    }

    #[test]
    fn a_measured_bonded_node_carries_its_score_and_source_epoch() {
        let view = verified(vec![entry(7, 10, Some("0.85"))], &[7]).routing_view();

        assert_eq!(view.nodes[&7], Some(entry(7, 10, Some("0.85"))));
    }

    #[test]
    fn a_bonded_node_never_measured_has_no_entry() {
        let view = verified(vec![entry(7, 10, Some("0.85"))], &[7, 8]).routing_view();

        assert_eq!(view.nodes[&8], None);
    }

    /// Cannot arise while the mixnet contract deletes a node's pointer as it unbonds, so this
    /// is a guard rather than an expected state.
    #[test]
    fn a_committed_entry_without_a_bond_is_left_out() {
        let verified = verified(vec![entry(9, 10, Some("0.85"))], &[7]);
        let view = verified.routing_view();

        assert!(!view.nodes.contains_key(&9));
        assert!(verified.entries.contains_key(&9));
    }

    #[test]
    fn an_unscoreable_entry_stays_in_the_view() {
        let view = verified(vec![entry(7, 10, None)], &[7]).routing_view();

        assert_eq!(view.nodes[&7], Some(entry(7, 10, None)));
    }

    #[test]
    fn an_old_entry_is_served_as_is() {
        let view = verified(vec![entry(7, 1, Some("0.85"))], &[7]).routing_view();

        assert_eq!(view.nodes[&7], Some(entry(7, 1, Some("0.85"))));
    }

    #[test]
    fn the_view_carries_the_verified_height() {
        let verified = verified(Vec::new(), &[7]);

        assert_eq!(verified.routing_view().height, verified.height);
    }
}
