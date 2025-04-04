// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::mixnet::packet_forwarding::global::is_global_ip;
use crate::node::routing_filter::RoutingFilter;
use arc_swap::ArcSwap;
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

impl RoutingFilter for NetworkRoutingFilter {
    fn should_route(&self, ip: IpAddr) -> bool {
        // only allow non-global ips on testnets
        if self.testnet_mode && !is_global_ip(&ip) {
            return true;
        }

        self.attempt_resolve(ip).should_route()
    }
}

#[derive(Clone)]
pub(crate) struct NetworkRoutingFilter {
    testnet_mode: bool,

    pub(crate) resolved: KnownNodes,

    // while this is technically behind a lock, it should not be called too often as once resolved it will
    // be present on the arcswap in either allowed or denied section
    pub(crate) pending: UnknownNodes,
}

impl NetworkRoutingFilter {
    pub(crate) fn new_empty(testnet_mode: bool) -> Self {
        NetworkRoutingFilter {
            testnet_mode,
            resolved: Default::default(),
            pending: Default::default(),
        }
    }

    pub(crate) fn attempt_resolve(&self, ip: IpAddr) -> Resolution {
        if self.resolved.inner.allowed.load().contains(&ip) {
            Resolution::Accept
        } else if self.resolved.inner.denied.load().contains(&ip) {
            Resolution::Deny
        } else {
            self.pending.try_insert(ip);
            Resolution::Unknown
        }
    }

    pub(crate) fn allowed_nodes_copy(&self) -> HashSet<IpAddr> {
        self.resolved.inner.allowed.load_full().as_ref().clone()
    }

    pub(crate) fn denied_nodes_copy(&self) -> HashSet<IpAddr> {
        self.resolved.inner.denied.load_full().as_ref().clone()
    }
}

#[derive(Clone, Default)]
pub(crate) struct UnknownNodes(Arc<RwLock<HashSet<IpAddr>>>);

impl UnknownNodes {
    fn try_insert(&self, ip: IpAddr) {
        // if we can immediately grab the lock to push it into the pending queue, amazing, let's do it
        // otherwise we can do it next time we see this ip
        // (if we can't hold the lock, it means it's being updated at this very moment which is actually a good thing)
        if let Ok(mut guard) = self.0.try_write() {
            guard.insert(ip);
        }
    }

    pub(crate) async fn clear(&self) {
        self.0.write().await.clear();
    }

    pub(crate) async fn nodes(&self) -> HashSet<IpAddr> {
        self.0.read().await.clone()
    }
}

// for now we don't care about keys, etc.
// we only want to know if given ip belongs to a known node
#[derive(Debug, Default, Clone)]
pub(crate) struct KnownNodes {
    inner: Arc<KnownNodesInner>,
}

#[derive(Debug, Default)]
struct KnownNodesInner {
    allowed: ArcSwap<HashSet<IpAddr>>,
    denied: ArcSwap<HashSet<IpAddr>>,
}

pub(crate) enum Resolution {
    Unknown,
    Deny,
    Accept,
}

impl From<bool> for Resolution {
    fn from(value: bool) -> Self {
        if value {
            Resolution::Accept
        } else {
            Resolution::Deny
        }
    }
}

impl Resolution {
    pub(crate) fn should_route(&self) -> bool {
        matches!(self, Resolution::Accept)
    }
}

impl KnownNodes {
    pub(crate) fn swap_allowed(&self, new: HashSet<IpAddr>) {
        self.inner.allowed.store(Arc::new(new))
    }

    pub(crate) fn swap_denied(&self, new: HashSet<IpAddr>) {
        self.inner.denied.store(Arc::new(new))
    }
}
