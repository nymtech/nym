// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::MAX_FALLBACK_LOOKBACK_EPOCHS;
use crate::{EpochId, KindMedians, ResolvedMedians};
use cosmwasm_std::StdResult;

/// The medians rewarding uses for a node at `epoch_id`, reading each lookup only when needed.
///
/// `last_known` is only a starting hint: it names the node's newest bundle, so the walk meets
/// the same bundle first with or without it.
pub fn resolve_source(
    epoch_id: EpochId,
    mut medians_at: impl FnMut(EpochId) -> StdResult<Option<KindMedians>>,
    newest_measured: impl FnOnce(EpochId) -> StdResult<Option<EpochId>>,
    last_known: impl FnOnce() -> StdResult<Option<EpochId>>,
) -> StdResult<Option<ResolvedMedians>> {
    if let Some(medians) = medians_at(epoch_id)? {
        return Ok(Some(ResolvedMedians { epoch_id, medians }));
    }
    // the requested epoch's own bundle is missing, so the source lies below it
    let Some(below) = epoch_id.checked_sub(1) else {
        return Ok(None);
    };
    let Some(newest) = newest_measured(epoch_id)? else {
        return Ok(None);
    };

    // the walk goes newest first, from `top` down, so it stops at the bundle rewarding saw at the
    // time
    let top = match last_known()? {
        // the pointer names the node's newest bundle, so the walk starts right on it
        Some(last_known) if last_known <= epoch_id => last_known,
        // data arrived after the requested epoch, or there is no pointer because the node was
        // never measured, has unbonded, or the caller keeps none: start at R, or just below the
        // request
        _ => below.min(newest),
    };
    let floor = newest.saturating_sub(MAX_FALLBACK_LOOKBACK_EPOCHS);
    for epoch_id in (floor..=top).rev() {
        if let Some(medians) = medians_at(epoch_id)? {
            return Ok(Some(ResolvedMedians { epoch_id, medians }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_contracts_common::Percent;
    use std::cell::Cell;
    use std::collections::BTreeSet;

    /// One node's bundles, and every epoch in which any node has a bundle (the node's included).
    struct Bundles {
        node: BTreeSet<EpochId>,
        any: BTreeSet<EpochId>,
    }

    impl Bundles {
        fn new(node: impl IntoIterator<Item = EpochId>) -> Self {
            let node: BTreeSet<_> = node.into_iter().collect();
            Bundles {
                any: node.clone(),
                node,
            }
        }

        fn with_others(mut self, others: impl IntoIterator<Item = EpochId>) -> Self {
            self.any.extend(others);
            self
        }

        /// Medians that tell bundles apart, so a test can see which one was returned.
        fn medians(epoch_id: EpochId) -> KindMedians {
            KindMedians {
                liveness: Some(Percent::from_percentage_value(u64::from(epoch_id % 101)).unwrap()),
                ..Default::default()
            }
        }

        fn resolve_with(
            &self,
            epoch_id: EpochId,
            last_known: Option<EpochId>,
        ) -> Option<ResolvedMedians> {
            resolve_source(
                epoch_id,
                |epoch_id| {
                    Ok(self
                        .node
                        .contains(&epoch_id)
                        .then(|| Self::medians(epoch_id)))
                },
                |at_or_before| Ok(self.any.range(..=at_or_before).next_back().copied()),
                || Ok(last_known),
            )
            .unwrap()
        }

        /// What the contract resolves: its pointer is the node's newest bundle.
        fn resolve(&self, epoch_id: EpochId) -> Option<EpochId> {
            let pointer = self.node.last().copied();
            self.resolve_with(epoch_id, pointer)
                .map(|source| source.epoch_id)
        }
    }

    #[test]
    fn uses_the_bundle_of_the_requested_epoch() {
        let bundles = Bundles::new([8, 10]);

        let source = bundles.resolve_with(10, Some(10)).unwrap();
        assert_eq!(source.epoch_id, 10);
        assert_eq!(source.medians, Bundles::medians(10));
    }

    #[test]
    fn falls_back_to_the_newest_earlier_bundle_within_the_lookback() {
        let bundles = Bundles::new([8, 12]);

        assert_eq!(bundles.resolve(10), Some(8));
        assert_eq!(bundles.resolve(11), Some(8));

        let source = bundles.resolve_with(11, Some(12)).unwrap();
        assert_eq!(source.medians, Bundles::medians(8));
    }

    #[test]
    fn does_not_reach_below_the_lookback() {
        let edge = 10 + MAX_FALLBACK_LOOKBACK_EPOCHS;
        let bundles = Bundles::new([10]).with_others(10..=edge + 1);

        assert_eq!(bundles.resolve(edge), Some(10));
        assert_eq!(bundles.resolve(edge + 1), None);
    }

    #[test]
    fn a_global_outage_freezes_every_node_at_its_last_bundle() {
        let bundles = Bundles::new([10]);

        assert_eq!(bundles.resolve(100), Some(10));
    }

    #[test]
    fn an_audit_across_a_long_outage_reproduces_what_was_resolved() {
        let bundles = Bundles::new([10, 60]);

        assert_eq!(bundles.resolve(40), Some(10));
    }

    #[test]
    fn resolves_nothing_before_the_first_bundle() {
        assert_eq!(Bundles::new([]).resolve(5), None);
        assert_eq!(Bundles::new([10]).resolve(5), None);
        assert_eq!(Bundles::new([10]).resolve(0), None);
    }

    #[test]
    fn a_hit_reads_nothing_but_the_requested_epoch() {
        let reads = Cell::new(0);
        let asked_for_r = Cell::new(false);
        let asked_for_pointer = Cell::new(false);

        let source = resolve_source(
            10,
            |epoch_id| {
                reads.set(reads.get() + 1);
                Ok((epoch_id == 10).then(KindMedians::default))
            },
            |_| {
                asked_for_r.set(true);
                Ok(Some(10))
            },
            || {
                asked_for_pointer.set(true);
                Ok(Some(10))
            },
        )
        .unwrap();

        assert_eq!(source.map(|source| source.epoch_id), Some(10));
        assert_eq!(reads.get(), 1);
        assert!(!asked_for_r.get());
        assert!(!asked_for_pointer.get());
    }

    /// The pointer only saves loads: over every layout of one node's bundles and of other nodes'
    /// bundles, at positions straddling the lookback boundaries, the walk resolves the same source
    /// with the pointer the contract holds as without one.
    #[test]
    fn the_pointer_hint_never_changes_the_source() {
        const POSITIONS: [EpochId; 8] = [0, 1, 2, 25, 26, 27, 50, 51];

        let layout = |mask: u32| {
            POSITIONS
                .iter()
                .enumerate()
                .filter(move |(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, epoch_id)| *epoch_id)
        };
        let requested: BTreeSet<EpochId> = POSITIONS
            .iter()
            .flat_map(|epoch_id| [*epoch_id, epoch_id + 1])
            .chain([100])
            .collect();

        for node_mask in 0..(1 << POSITIONS.len()) {
            for others_mask in 0..(1 << POSITIONS.len()) {
                let bundles = Bundles::new(layout(node_mask)).with_others(layout(others_mask));
                let pointer = bundles.node.last().copied();

                for &epoch_id in &requested {
                    assert_eq!(
                        bundles.resolve_with(epoch_id, pointer),
                        bundles.resolve_with(epoch_id, None),
                        "node bundles {:?}, any bundles {:?}, requested epoch {epoch_id}",
                        bundles.node,
                        bundles.any,
                    );
                }
            }
        }
    }
}
