// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_directory_contract_common::KnownLabel;
use nym_directory_types::SphinxKeys;
use prost::Message;

/// The closed set of payloads this node publishes to the directory contract - one
/// variant per [`KnownLabel`]. A closed enum (rather than an open trait) gives
/// compiler-exhaustiveness against the contract's label whitelist: every known label
/// must be handled here, and the label<->payload correspondence becomes a property of
/// the type.
// `EnumIter` (test-only) lets the label-mapping test iterate every variant, so a
// backfilled payload is covered without maintaining a hand-written variant list.
#[cfg_attr(test, derive(strum_macros::EnumIter))]
pub(crate) enum DirectoryPayload {
    /// The node's rotation-tagged sphinx keys, published under [`KnownLabel::SphinxKeys`].
    SphinxKeys(SphinxKeys),
}

impl DirectoryPayload {
    /// The contract label this payload is written under.
    pub(crate) fn label(&self) -> KnownLabel {
        match self {
            DirectoryPayload::SphinxKeys(_) => KnownLabel::SphinxKeys,
        }
    }

    /// The canonical `data` bytes for this entry - the exact bytes a reader decodes.
    pub(crate) fn to_canonical_bytes(&self) -> Vec<u8> {
        match self {
            DirectoryPayload::SphinxKeys(payload) => payload.encode_to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use strum::IntoEnumIterator;

    #[test]
    fn every_variant_maps_to_a_distinct_known_label() {
        // `EnumIter` yields every variant automatically, so a backfilled payload is
        // covered here without anyone remembering to update a hand-maintained list.
        let labels: Vec<KnownLabel> = DirectoryPayload::iter().map(|p| p.label()).collect();
        let unique: BTreeSet<KnownLabel> = labels.iter().copied().collect();

        // no two payload variants share a label
        assert_eq!(
            labels.len(),
            unique.len(),
            "two DirectoryPayload variants map to the same KnownLabel"
        );

        // and the variants correspond exactly to the contract's known labels, so a
        // backfilled payload can neither miss a label nor invent one outside the catalog
        let known: BTreeSet<KnownLabel> = KnownLabel::ALL.iter().copied().collect();
        assert_eq!(
            unique, known,
            "DirectoryPayload variants must correspond 1:1 to KnownLabel::ALL"
        );
    }
}
