// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_directory_contract_common::KnownLabel;
use nym_topology::NodeId;
use std::collections::{BTreeMap, BTreeSet};

/// Per-run state that exists only once startup preflight has resolved the node's
/// `node_id` and confirmed it can write.
pub(crate) struct ActiveSession {
    pub(crate) node_id: NodeId,

    /// The next sequence the contract expects this node to sign with (gap-free).
    pub(crate) next_sequence: u64,

    /// Snapshot of what this node currently has published on-chain, keyed by label - the
    /// basis for reconcile-before-write (skip a write whose canonical bytes are unchanged).
    pub(crate) published: BTreeMap<KnownLabel, Vec<u8>>,

    /// The contract's current label whitelist, restricted to labels that parse to a
    /// `KnownLabel`. Refreshed each sweep; a write to a label absent here is skipped.
    pub(crate) whitelist: BTreeSet<KnownLabel>,

    /// Contract whitelist labels that do not parse to a `KnownLabel` and have already been
    /// warned about, so the "node binary may be behind" warning fires once per unchanged
    /// state rather than every refresh (5.3).
    pub(crate) warned_unknown_labels: BTreeSet<String>,
}

impl ActiveSession {
    /// Whether the contract currently whitelists `label`. A write to a non-whitelisted label
    /// is rejected on-chain, so the sweep (and, later, the event path) skip it rather than
    /// issue a doomed transaction.
    pub(crate) fn label_is_writable(&self, label: KnownLabel) -> bool {
        self.whitelist.contains(&label)
    }
}
