// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use arc_swap::ArcSwap;
use nym_sphinx::addressing::clients::Recipient;
use nym_topology::{NymRouteProvider, NymTopology, NymTopologyError, NymTopologyMetadata};
use nym_validator_client::models::KeyRotationId;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

#[derive(Debug)]
pub struct TopologyAccessorInner {
    controlled_manually: AtomicBool,
    released_manual_control: Notify,

    /// Client-wide routing policy rather than a property of any particular network view,
    /// hence it's kept out of the swapped value and applied to every route provider handed out.
    ignore_egress_epoch_roles: bool,

    // the topology is read for every single packet that gets generated, while it's only written
    // whenever the refresher obtains fresh network information, i.e. every few minutes,
    // hence the read path is kept wait-free
    topology: ArcSwap<NymTopology>,
}

impl TopologyAccessorInner {
    fn new(ignore_egress_epoch_roles: bool) -> Self {
        TopologyAccessorInner {
            controlled_manually: AtomicBool::new(false),
            released_manual_control: Notify::new(),
            ignore_egress_epoch_roles,
            topology: ArcSwap::from_pointee(NymTopology::default()),
        }
    }

    fn update(&self, new: Option<NymTopology>) {
        self.topology.store(Arc::new(new.unwrap_or_default()))
    }

    /// Combines a snapshot of the current network view with the configured routing policy.
    fn route_provider(&self) -> NymRouteProvider {
        NymRouteProvider::new(self.topology.load_full(), self.ignore_egress_epoch_roles)
    }
}

#[derive(Clone, Debug)]
pub struct TopologyAccessor {
    inner: Arc<TopologyAccessorInner>,
}

impl TopologyAccessor {
    pub fn new(ignore_egress_epoch_roles: bool) -> Self {
        TopologyAccessor {
            inner: Arc::new(TopologyAccessorInner::new(ignore_egress_epoch_roles)),
        }
    }

    pub fn controlled_manually(&self) -> bool {
        self.inner.controlled_manually.load(Ordering::SeqCst)
    }

    /// Attempts to obtain a snapshot of the current topology that can be used for constructing
    /// a packet towards the (optional) packet recipient, with acks getting back to the ack recipient.
    ///
    /// The returned snapshot is unaffected by any topology updates happening while it's being held,
    /// so a packet is always constructed against a consistent view of the network.
    pub(crate) fn try_get_valid_topology(
        &self,
        ack_recipient: &Recipient,
        packet_recipient: Option<&Recipient>,
    ) -> Result<NymRouteProvider, NymTopologyError> {
        let route_provider = self.inner.route_provider();
        let topology = &route_provider.topology;

        // 1. Have we managed to get anything from the refresher, i.e. have the nym-api queries gone through?
        topology.ensure_not_empty()?;

        // 2. does the topology have a node on each mixing layer?
        topology.ensure_minimally_routable()?;

        // 3. does it contain OUR gateway (so that we could create an ack packet)?
        let _ = route_provider.egress_by_identity(ack_recipient.gateway())?;

        // 4. for our target recipient, does it contain THEIR gateway (so that we send anything over?)
        if let Some(recipient) = packet_recipient {
            let _ = route_provider.egress_by_identity(recipient.gateway())?;
        }

        Ok(route_provider)
    }

    pub(crate) fn update_global_topology(&self, new_topology: Option<NymTopology>) {
        self.inner.update(new_topology);
    }

    pub(crate) async fn wait_for_released_manual_control(&self) {
        self.inner.released_manual_control.notified().await
    }

    pub fn current_route_provider(&self) -> Option<NymRouteProvider> {
        let provider = self.inner.route_provider();
        if provider.topology.is_empty() {
            None
        } else {
            Some(provider)
        }
    }

    // helper for the fns below that only need to peek at the topology rather than route through it
    fn current_topology_guard(&self) -> Option<arc_swap::Guard<Arc<NymTopology>>> {
        let topology = self.inner.topology.load();
        if topology.is_empty() {
            None
        } else {
            Some(topology)
        }
    }

    pub fn current_mixnet_epoch_id(&self) -> Option<u32> {
        Some(self.current_metadata()?.absolute_epoch_id)
    }

    pub fn current_key_rotation_id(&self) -> Option<KeyRotationId> {
        Some(self.current_metadata()?.key_rotation_id)
    }

    pub fn current_metadata(&self) -> Option<NymTopologyMetadata> {
        Some(self.current_topology_guard()?.metadata())
    }

    pub fn manually_change_topology(&self, new_topology: NymTopology) {
        self.inner.controlled_manually.store(true, Ordering::SeqCst);
        self.inner.update(Some(new_topology));
    }

    pub fn release_manual_control(&self) {
        self.inner
            .controlled_manually
            .store(false, Ordering::SeqCst);
        self.inner.released_manual_control.notify_waiters();
    }

    // only used by the client at startup to get a slightly more reasonable error message
    // (currently displays as unused because health checker is disabled due to required changes)
    pub fn ensure_is_routable(&self) -> Result<(), NymTopologyError> {
        self.inner.topology.load().ensure_minimally_routable()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_topology::{CachedEpochRewardedSet, NodeId, RoutingNode, SupportedRoles};
    use time::OffsetDateTime;

    fn dummy_node(node_id: NodeId) -> RoutingNode {
        RoutingNode {
            node_id,
            mix_host: "1.2.3.4:1789".parse().unwrap(),
            entry: None,
            identity_key: "GLdR2NRVZBiCoCbv4fNqt9wUJZAnNjGXHkx3TjVAUzrK"
                .parse()
                .unwrap(),
            sphinx_key: "CBmYewWf43iarBq349KhbfYMc9ys2ebXWd4Vp4CLQ5Rq"
                .parse()
                .unwrap(),
            supported_roles: SupportedRoles {
                mixnode: true,
                mixnet_entry: false,
                mixnet_exit: false,
            },
        }
    }

    // the topology is only distinguished by its epoch id - it's not meant to be routable
    fn topology_with_epoch(absolute_epoch_id: u32) -> NymTopology {
        let node = dummy_node(1);
        let mut rewarded_set = CachedEpochRewardedSet::default();
        rewarded_set.layer1.insert(node.node_id);

        let metadata = NymTopologyMetadata::new(0, absolute_epoch_id, OffsetDateTime::now_utc());
        NymTopology::new(metadata, rewarded_set, vec![node])
    }

    #[test]
    fn update_is_visible_to_subsequent_readers() {
        let accessor = TopologyAccessor::new(false);
        accessor.update_global_topology(Some(topology_with_epoch(42)));

        assert_eq!(Some(42), accessor.current_mixnet_epoch_id());
    }

    #[test]
    fn existing_snapshot_is_unaffected_by_updates() {
        let accessor = TopologyAccessor::new(false);
        accessor.update_global_topology(Some(topology_with_epoch(42)));

        let snapshot = accessor.current_route_provider().unwrap();
        accessor.update_global_topology(Some(topology_with_epoch(43)));

        // whoever obtained the snapshot keeps using the topology it was created with...
        assert_eq!(42, snapshot.absolute_epoch_id());

        // ... while a fresh read sees the updated one
        assert_eq!(Some(43), accessor.current_mixnet_epoch_id());
    }

    #[test]
    fn configured_routing_policy_is_applied_to_every_provider() {
        let accessor = TopologyAccessor::new(true);

        accessor.update_global_topology(Some(topology_with_epoch(42)));
        assert!(
            accessor
                .current_route_provider()
                .unwrap()
                .ignore_egress_epoch_roles
        );

        // an empty topology is not exposed via the public getter, but it gets the same policy
        accessor.update_global_topology(None);
        assert!(accessor.inner.route_provider().ignore_egress_epoch_roles);
    }
}
