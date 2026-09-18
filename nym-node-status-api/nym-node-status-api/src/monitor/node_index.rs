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

#[derive(Debug, Default)]
pub(crate) struct NodeIndex {
    by_identity: HashMap<IdentityKey, NodeId>,
}

impl NodeIndex {
    pub(crate) fn node_id(&self, identity_key: IdentityKeyRef<'_>) -> Option<NodeId> {
        self.by_identity.get(identity_key).copied()
    }
}

impl FromIterator<(IdentityKey, NodeId)> for NodeIndex {
    fn from_iter<I: IntoIterator<Item = (IdentityKey, NodeId)>>(entries: I) -> Self {
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
