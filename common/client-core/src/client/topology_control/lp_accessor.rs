// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The LP network view, held beside the legacy one.
//!
//! Deliberately a separate accessor rather than a second field on [`TopologyAccessor`]: the two
//! describe different networks - a node that published no LP details is in one and not the other -
//! and a client sends on one path or the other, never a route spanning both. Each is refreshed from
//! its own feed and fails on its own, so a broken LP listing leaves the legacy path working and
//! vice versa.
//!
//! [`TopologyAccessor`]: super::TopologyAccessor

use arc_swap::ArcSwap;
use nym_topology::lp::{LpRouteProvider, LpTopology};
use nym_topology::{NymTopologyError, NymTopologyMetadata};
use nym_validator_client::models::KeyRotationId;
use std::sync::Arc;

#[derive(Debug)]
struct LpTopologyAccessorInner {
    /// Client-wide routing policy rather than a property of any particular network view, so it is
    /// kept out of the swapped value and applied to every provider handed out.
    ignore_egress_epoch_roles: bool,

    // read for every packet, written only when the refresher gets fresh network information, so
    // the read path is kept wait-free
    topology: ArcSwap<LpTopology>,
}

impl LpTopologyAccessorInner {
    fn new(ignore_egress_epoch_roles: bool) -> Self {
        LpTopologyAccessorInner {
            ignore_egress_epoch_roles,
            topology: ArcSwap::from_pointee(LpTopology::default()),
        }
    }

    fn update(&self, new: Option<LpTopology>) {
        self.topology.store(Arc::new(new.unwrap_or_default()))
    }

    fn route_provider(&self) -> LpRouteProvider {
        LpRouteProvider::new(self.topology.load_full(), self.ignore_egress_epoch_roles)
    }
}

#[derive(Clone, Debug)]
pub struct LpTopologyAccessor {
    inner: Arc<LpTopologyAccessorInner>,
}

impl LpTopologyAccessor {
    pub fn new(ignore_egress_epoch_roles: bool) -> Self {
        LpTopologyAccessor {
            inner: Arc::new(LpTopologyAccessorInner::new(ignore_egress_epoch_roles)),
        }
    }

    /// A snapshot to build routes against, or `None` while nothing has been fetched yet.
    ///
    /// Empty rather than stale is the only state worth distinguishing here: an LP route through a
    /// node that turns out not to speak LP is a black hole, so the caller is better off knowing it
    /// has no network than routing into one.
    pub fn current_route_provider(&self) -> Option<LpRouteProvider> {
        let provider = self.inner.route_provider();
        if provider.topology.is_empty() {
            None
        } else {
            Some(provider)
        }
    }

    pub(crate) fn update_global_topology(&self, new_topology: Option<LpTopology>) {
        self.inner.update(new_topology);
    }

    fn current_topology_guard(&self) -> Option<arc_swap::Guard<Arc<LpTopology>>> {
        let topology = self.inner.topology.load();
        if topology.is_empty() {
            None
        } else {
            Some(topology)
        }
    }

    pub fn current_metadata(&self) -> Option<NymTopologyMetadata> {
        Some(self.current_topology_guard()?.metadata())
    }

    pub fn current_key_rotation_id(&self) -> Option<KeyRotationId> {
        Some(self.current_metadata()?.key_rotation_id)
    }

    /// Whether a route can be built through this view at all.
    ///
    /// A thin LP rollout can leave a mix layer empty while the view still holds plenty of nodes,
    /// so a caller that wants to report the network as usable has to ask this rather than count.
    pub fn ensure_is_routable(&self) -> Result<(), NymTopologyError> {
        self.inner.topology.load().ensure_minimally_routable()
    }

    pub fn manually_change_topology(&self, new_topology: LpTopology) {
        self.inner.update(Some(new_topology));
    }
}
