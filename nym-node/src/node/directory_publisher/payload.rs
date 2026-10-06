// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_directory_contract_common::KnownLabel;
use nym_directory_types::{
    LewesProtocolDetails, MixnetServiceProviders, NodeDescription, NodeInformation, SphinxKeys,
    Wireguard,
};
use prost::Message;

/// Per-payload publishing policy: when an event-driven write must be held back, and
/// whether the entry already on chain still satisfies the payload.
pub(crate) trait ReconcilePayload: Message + Sized {
    /// Whether event-driven writes of this payload fire at the same instant on every node
    /// (a network-wide schedule, e.g. the sphinx key pre-announce at an epoch boundary) and
    /// so must be held for a random delay before hitting the chain. Deliberately not
    /// defaulted: a synchronised producer that skips the delay lands the whole network's
    /// writes in the same few blocks.
    fn network_synchronised_events(&self) -> bool;

    /// Whether `published`, the entry already on chain under this payload's label, makes a
    /// write unnecessary. Byte equality with the canonical encoding by default; a payload
    /// whose on-chain entry may legitimately hold more than the node's live state overrides
    /// it.
    fn is_satisfied_by(&self, published: &[u8]) -> bool {
        self.encode_to_vec() == published
    }
}

impl ReconcilePayload for NodeDescription {
    fn network_synchronised_events(&self) -> bool {
        false
    }
}

impl ReconcilePayload for MixnetServiceProviders {
    fn network_synchronised_events(&self) -> bool {
        false
    }
}

impl ReconcilePayload for Wireguard {
    fn network_synchronised_events(&self) -> bool {
        false
    }
}

impl ReconcilePayload for NodeInformation {
    fn network_synchronised_events(&self) -> bool {
        false
    }
}

impl ReconcilePayload for LewesProtocolDetails {
    fn network_synchronised_events(&self) -> bool {
        false
    }
}

impl ReconcilePayload for SphinxKeys {
    // The rotation controller pre-announces at an epoch boundary, the same instant on
    // every node.
    fn network_synchronised_events(&self) -> bool {
        true
    }

    // Stale only if a live rotation's key is absent or differs on chain. Extra keys are
    // tolerated: the previous rotation's key stays published after the local purge, so the
    // entry is written once per rotation, at pre-announce, whose full replace drops the
    // oldest key and bounds the entry at two.
    fn is_satisfied_by(&self, published: &[u8]) -> bool {
        SphinxKeys::decode(published).is_ok_and(|published| {
            self.keys
                .iter()
                .all(|(rotation, key)| published.keys.get(rotation) == Some(key))
        })
    }
}

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

    /// The node's operator-provided description.
    NodeDescription(NodeDescription),

    /// The node's mixnet service-provider addresses.
    MixnetServiceProviders(MixnetServiceProviders),

    /// The node's wireguard connection details.
    Wireguard(Wireguard),

    /// The node's general self-reported information.
    NodeInformation(NodeInformation),

    /// The node's Lewes Protocol connection details.
    LewesProtocolDetails(LewesProtocolDetails),
}

impl DirectoryPayload {
    /// The contract label this payload is written under.
    pub(crate) fn label(&self) -> KnownLabel {
        match self {
            DirectoryPayload::SphinxKeys(_) => KnownLabel::SphinxKeys,
            DirectoryPayload::NodeDescription(_) => KnownLabel::NodeDescription,
            DirectoryPayload::MixnetServiceProviders(_) => KnownLabel::MixnetServiceProviders,
            DirectoryPayload::Wireguard(_) => KnownLabel::Wireguard,
            DirectoryPayload::NodeInformation(_) => KnownLabel::NodeInformation,
            DirectoryPayload::LewesProtocolDetails(_) => KnownLabel::LewesProtocolDetails,
        }
    }

    /// The canonical `data` bytes for this entry - the exact bytes a reader decodes.
    pub(crate) fn to_canonical_bytes(&self) -> Vec<u8> {
        match self {
            DirectoryPayload::SphinxKeys(payload) => payload.encode_to_vec(),
            DirectoryPayload::NodeDescription(payload) => payload.encode_to_vec(),
            DirectoryPayload::MixnetServiceProviders(payload) => payload.encode_to_vec(),
            DirectoryPayload::Wireguard(payload) => payload.encode_to_vec(),
            DirectoryPayload::NodeInformation(payload) => payload.encode_to_vec(),
            DirectoryPayload::LewesProtocolDetails(payload) => payload.encode_to_vec(),
        }
    }

    /// See [`ReconcilePayload::network_synchronised_events`].
    pub(crate) fn network_synchronised_events(&self) -> bool {
        match self {
            DirectoryPayload::SphinxKeys(payload) => payload.network_synchronised_events(),
            DirectoryPayload::NodeDescription(payload) => payload.network_synchronised_events(),
            DirectoryPayload::MixnetServiceProviders(payload) => {
                payload.network_synchronised_events()
            }
            DirectoryPayload::Wireguard(payload) => payload.network_synchronised_events(),
            DirectoryPayload::NodeInformation(payload) => payload.network_synchronised_events(),
            DirectoryPayload::LewesProtocolDetails(payload) => {
                payload.network_synchronised_events()
            }
        }
    }

    /// See [`ReconcilePayload::is_satisfied_by`].
    pub(crate) fn is_satisfied_by(&self, published: &[u8]) -> bool {
        match self {
            DirectoryPayload::SphinxKeys(payload) => payload.is_satisfied_by(published),
            DirectoryPayload::NodeDescription(payload) => payload.is_satisfied_by(published),
            DirectoryPayload::MixnetServiceProviders(payload) => payload.is_satisfied_by(published),
            DirectoryPayload::Wireguard(payload) => payload.is_satisfied_by(published),
            DirectoryPayload::NodeInformation(payload) => payload.is_satisfied_by(published),
            DirectoryPayload::LewesProtocolDetails(payload) => payload.is_satisfied_by(published),
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
