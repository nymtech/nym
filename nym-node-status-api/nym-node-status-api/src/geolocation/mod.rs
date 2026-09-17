// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What one verified read of the geolocation contract leaves behind for the HTTP layer.

use arc_swap::ArcSwap;
use nym_geolocation_contract_common::payload;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::Height;
use std::collections::HashMap;
use std::sync::Arc;

/// Every location the resolution policy answered for at one height.
///
/// Replaced whole, never edited in place: the digest proof establishes that this set is
/// coherent at `height`, and serving entries from two reads together would discard that.
#[derive(Debug)]
pub(crate) struct GeoSnapshot {
    pub(crate) height: Height,

    /// The contract payload type rather than a shape of our own: both consumers already
    /// convert from it, and a local shape would only lose the explicit absence of
    /// coordinates on the way through.
    pub(crate) locations: HashMap<NodeId, payload::Location>,
}

impl GeoSnapshot {
    /// The cold-start value, held until the first refresh succeeds. Height `0` is a sentinel
    /// for "nothing read yet" - no refresh ever publishes one, because an empty snapshot
    /// empties the dVPN directory in one step.
    pub(crate) fn empty() -> Self {
        Self {
            height: Height::from(0u32),
            locations: HashMap::new(),
        }
    }
}

/// The snapshot as the monitor and the HTTP layer share it. Cheap to clone, and every clone
/// shares one cell, so a store through any of them is what all the others then load.
///
/// One pointer store replaces the whole value, so a reader gets either the new snapshot or
/// the previous one and never a mix of the two.
#[derive(Clone)]
pub(crate) struct GeoSnapshotHandle {
    inner: Arc<ArcSwap<GeoSnapshot>>,
}

impl GeoSnapshotHandle {
    /// Starts out holding [`GeoSnapshot::empty`].
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(GeoSnapshot::empty())),
        }
    }

    /// The snapshot as it stands. Synchronous and cheap, but one load still serves a whole
    /// response: loading per node would reintroduce exactly the incoherence the single
    /// height rules out.
    pub(crate) fn load(&self) -> Arc<GeoSnapshot> {
        self.inner.load_full()
    }

    /// Publish `snapshot` in place of the held one.
    pub(crate) fn store(&self, snapshot: GeoSnapshot) {
        self.inner.store(Arc::new(snapshot));
    }
}
