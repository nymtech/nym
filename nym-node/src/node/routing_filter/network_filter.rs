// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::routing_filter::RoutingFilter;
use arc_swap::ArcSwap;
use nym_bin_common::ip_check::is_global_ip;
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

impl RoutingFilter for NetworkRoutingFilter {
    fn should_route(&self, ip: IpAddr, is_network_monitor_packet: bool) -> bool {
        // only allow non-global ips on testnets
        if self.testnet_mode && !is_global_ip(&ip) {
            return true;
        }

        self.attempt_resolve(ip, is_network_monitor_packet)
            .should_route()
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

    #[must_use]
    pub(crate) fn with_known_network_monitors(
        mut self,
        known_network_monitors: HashSet<IpAddr>,
    ) -> Self {
        self.resolved.network_monitors = DeclaredNetworkMonitors::new(known_network_monitors);
        self
    }

    pub(crate) fn known_network_monitors_handle(&self) -> DeclaredNetworkMonitors {
        self.resolved.network_monitors.clone()
    }

    pub(crate) fn attempt_resolve(
        &self,
        ip: IpAddr,
        is_network_monitor_packet: bool,
    ) -> Resolution {
        // if packet has come from a network monitor it can ONLY go to another network monitor
        if is_network_monitor_packet {
            return if self.resolved.network_monitors.is_known(&ip) {
                Resolution::Accept
            } else {
                Resolution::Deny
            };
        }

        if self.resolved.nym_nodes.inner.allowed.load().contains(&ip) {
            // accept any traffic to known and resolved nym-nodes
            Resolution::Accept
        } else if self.resolved.nym_nodes.inner.denied.load().contains(&ip) {
            // deny any traffic to confirmed non-nym nodes
            Resolution::Deny
        } else if self.resolved.network_monitors.is_known(&ip) {
            // accept any traffic to known network monitors
            Resolution::Accept
        } else {
            // put any unknown destinations into resolution queue
            self.pending.try_insert(ip);
            Resolution::Unknown
        }
    }

    pub(crate) fn allowed_nodes_copy(&self) -> HashSet<IpAddr> {
        self.resolved.nym_nodes.clone_allowed()
    }

    pub(crate) fn denied_nodes_copy(&self) -> HashSet<IpAddr> {
        self.resolved.nym_nodes.clone_denied()
    }
}

/// Temporary queue of IP addresses that need resolution (are they Nym nodes or not?).
///
/// # Behaviour
///
/// - Packets from unknown IPs are denied initially and the IP is queued here
/// - A background task periodically processes this queue via nym-api lookups
/// - Once resolved, IPs are moved to either `allowed` or `denied` sets
///
/// # Lock Strategy
///
/// Uses `try_insert()` to avoid blocking the packet processing path:
/// - If lock is immediately available: insert the IP
/// - If lock is contended: skip insertion, will retry on next packet from same IP
/// - This is acceptable because resolution happens periodically anyway
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
#[derive(Debug, Clone, Default)]
pub(crate) struct KnownNodes {
    nym_nodes: KnownNymNodes,
    network_monitors: DeclaredNetworkMonitors,
}

impl KnownNodes {
    pub(crate) fn swap_allowed(&self, new: HashSet<IpAddr>) {
        self.nym_nodes.swap_allowed(new)
    }

    pub(crate) fn swap_denied(&self, new: HashSet<IpAddr>) {
        self.nym_nodes.swap_denied(new)
    }
}

/// Thread-safe, lock-free storage for authorised Network Monitor agents IP addresses.
///
/// # Concurrency Strategy
///
/// Uses `ArcSwap` for lock-free reads on the hot path (packet processing). Writes are rare
/// (only when blockchain authorisation events occur)
/// and involve cloning the HashSet, but this is acceptable because:
/// - Network monitor authorisations change extremely infrequently (on orchestrator startup with >5s per block)
/// - Read performance is critical (happens on every packet from unknown IPs)
/// - The HashSet is typically very small (<100 entries)
///
/// # Cloning
///
/// Cloning `DeclaredNetworkMonitors` is cheap (only clones the `Arc`), not the underlying data.
#[derive(Clone, Debug, Default)]
pub(crate) struct DeclaredNetworkMonitors {
    inner: Arc<DeclaredNetworkMonitorsInner>,
}

impl DeclaredNetworkMonitors {
    pub(crate) fn new(known: HashSet<IpAddr>) -> Self {
        Self {
            inner: Arc::new(DeclaredNetworkMonitorsInner {
                known: ArcSwap::from_pointee(known),
            }),
        }
    }

    fn swap(&self, new: HashSet<IpAddr>) {
        self.inner.known.store(Arc::new(new))
    }

    pub(crate) fn add_known(&self, address: IpAddr) {
        if self.is_known(&address) {
            return;
        }
        let mut known = self.inner.known.load().as_ref().clone();
        known.insert(address);
        self.swap(known);
    }

    pub(crate) fn remove_known(&self, address: IpAddr) {
        if !self.is_known(&address) {
            return;
        }
        let mut known = self.inner.known.load().as_ref().clone();
        known.remove(&address);
        self.swap(known);
    }

    pub(crate) fn reset(&self) {
        self.swap(HashSet::new())
    }

    pub(crate) fn is_known(&self, address: &IpAddr) -> bool {
        self.inner.known.load().contains(address)
    }
}

#[derive(Debug, Default)]
struct DeclaredNetworkMonitorsInner {
    known: ArcSwap<HashSet<IpAddr>>,
}

#[derive(Clone, Debug, Default)]
struct KnownNymNodes {
    inner: Arc<KnownNymNodesInner>,
}

impl KnownNymNodes {
    fn clone_allowed(&self) -> HashSet<IpAddr> {
        self.inner.allowed.load_full().as_ref().clone()
    }

    fn clone_denied(&self) -> HashSet<IpAddr> {
        self.inner.denied.load_full().as_ref().clone()
    }

    fn swap_allowed(&self, new: HashSet<IpAddr>) {
        self.inner.allowed.store(Arc::new(new))
    }

    fn swap_denied(&self, new: HashSet<IpAddr>) {
        self.inner.denied.store(Arc::new(new))
    }
}

#[derive(Debug, Default)]
struct KnownNymNodesInner {
    allowed: ArcSwap<HashSet<IpAddr>>,
    denied: ArcSwap<HashSet<IpAddr>>,
}

/// Result of attempting to resolve whether an IP address should be allowed to route packets.
///
/// # Semantics
///
/// - `Accept`: IP is a known Nym node OR authorised network monitor - route the packet
/// - `Deny`: IP has been confirmed as NOT a Nym node - drop the packet
/// - `Unknown`: IP hasn't been resolved yet - queue for lookup but DENY the packet
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
