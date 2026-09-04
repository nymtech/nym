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

    // the topology is read for every single packet that gets generated, while it's only written
    // whenever the refresher obtains fresh network information, i.e. every few minutes,
    // hence the read path is kept wait-free
    topology: ArcSwap<NymRouteProvider>,
}

impl TopologyAccessorInner {
    fn new(initial: NymRouteProvider) -> Self {
        TopologyAccessorInner {
            controlled_manually: AtomicBool::new(false),
            released_manual_control: Notify::new(),
            topology: ArcSwap::from_pointee(initial),
        }
    }

    fn update(&self, new: Option<NymTopology>) {
        // Only the topology is updated, the flag stays
        let ignore_egress_epoch_roles = self.topology.load().ignore_egress_epoch_roles;

        let updated = match new {
            Some(topology) => NymRouteProvider::new(topology, ignore_egress_epoch_roles),
            None => NymRouteProvider::new_empty(ignore_egress_epoch_roles),
        };

        self.topology.store(Arc::new(updated))
    }
}

#[derive(Clone, Debug)]
pub struct TopologyAccessor {
    inner: Arc<TopologyAccessorInner>,
}

impl TopologyAccessor {
    pub fn new(ignore_egress_epoch_roles: bool) -> Self {
        TopologyAccessor {
            inner: Arc::new(TopologyAccessorInner::new(NymRouteProvider::new_empty(
                ignore_egress_epoch_roles,
            ))),
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
    ) -> Result<Arc<NymRouteProvider>, NymTopologyError> {
        let route_provider = self.inner.topology.load_full();
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

    pub fn current_route_provider(&self) -> Option<Arc<NymRouteProvider>> {
        let provider = self.inner.topology.load_full();
        if provider.topology.is_empty() {
            None
        } else {
            Some(provider)
        }
    }

    pub fn current_mixnet_epoch_id(&self) -> Option<u32> {
        Some(self.current_route_provider()?.absolute_epoch_id())
    }

    pub fn current_key_rotation_id(&self) -> Option<KeyRotationId> {
        Some(self.current_route_provider()?.current_key_rotation())
    }

    pub fn current_metadata(&self) -> Option<NymTopologyMetadata> {
        Some(self.current_route_provider()?.metadata())
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
        self.inner
            .topology
            .load()
            .topology
            .ensure_minimally_routable()
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
    fn ignore_egress_epoch_roles_survives_updates() {
        let accessor = TopologyAccessor::new(true);

        accessor.update_global_topology(Some(topology_with_epoch(42)));
        assert!(
            accessor
                .current_route_provider()
                .unwrap()
                .ignore_egress_epoch_roles
        );

        // an empty topology is not exposed via the public getters, hence we have to reach inside
        accessor.update_global_topology(None);
        assert!(accessor.inner.topology.load().ignore_egress_epoch_roles);
    }
}
