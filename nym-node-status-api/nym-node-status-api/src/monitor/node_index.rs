// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Identity to node id, for the response paths that hold one and need the other.
//!
//! The `gateways` table is keyed by base58 identity and carries no node id, while everything
//! read from chain is keyed by node id. The monitor already holds both for every described
//! node, so it publishes the mapping rather than making each reader join for it.

use arc_swap::ArcSwap;
use nym_contracts_common::{IdentityKey, IdentityKeyRef};
use nym_validator_client::client::NodeId;
use std::collections::HashMap;
use std::sync::Arc;

/// What a response path needs about a node that its own row does not carry.
#[derive(Debug, Clone)]
pub(crate) struct IndexedNode {
    pub(crate) node_id: NodeId,

    /// The node's first declared host IP, empty when it announces only a hostname. Carried here
    /// because no IP address is written on chain in any form, so a response that has always
    /// shown one has nowhere else to get it.
    pub(crate) ip_address: String,
}

#[derive(Debug, Default)]
pub(crate) struct NodeIndex {
    by_identity: HashMap<IdentityKey, IndexedNode>,
}

impl NodeIndex {
    pub(crate) fn get(&self, identity_key: IdentityKeyRef<'_>) -> Option<&IndexedNode> {
        self.by_identity.get(identity_key)
    }
}

impl FromIterator<(IdentityKey, IndexedNode)> for NodeIndex {
    fn from_iter<I: IntoIterator<Item = (IdentityKey, IndexedNode)>>(entries: I) -> Self {
        NodeIndex {
            by_identity: entries.into_iter().collect(),
        }
    }
}

/// Cheap to clone, and every clone shares one cell, exactly like the geolocation snapshot's
/// handle. Replaced whole on each monitor cycle rather than edited in place.
#[derive(Clone, Default)]
pub(crate) struct NodeIndexHandle {
    inner: Arc<ArcSwap<NodeIndex>>,
}

impl NodeIndexHandle {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn load(&self) -> Arc<NodeIndex> {
        self.inner.load_full()
    }

    pub(crate) fn store(&self, index: NodeIndex) {
        self.inner.store(Arc::new(index));
    }
}
