// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_directory_contract_common::KnownLabel;
use nym_topology::NodeId;
use std::collections::BTreeMap;

/// Per-run state that exists only once startup preflight has resolved the node's
/// `node_id` and confirmed it can write.
pub(crate) struct ActiveSession {
    pub(crate) node_id: NodeId,

    /// The next sequence the contract expects this node to sign with (gap-free).
    pub(crate) next_sequence: u64,

    /// Snapshot of what this node currently has published on-chain, keyed by label - the
    /// basis for reconcile-before-write (skip a write whose canonical bytes are unchanged).
    pub(crate) published: BTreeMap<KnownLabel, Vec<u8>>,
}
