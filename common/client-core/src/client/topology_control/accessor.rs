// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx::addressing::clients::Recipient;
use nym_topology::{NymRouteProvider, NymTopology, NymTopologyError};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock, RwLockReadGuard};

#[derive(Debug)]
pub struct TopologyAccessorInner {
    controlled_manually: AtomicBool,
    released_manual_control: Notify,
    // `RwLock` *seems to* be the better approach for this as write access is only requested every
    // few seconds, while reads are needed every single packet generated.
    // However, proper benchmarks will be needed to determine if `RwLock` is indeed a better
    // approach than a `Mutex`
    topology: RwLock<NymRouteProvider>,
}

impl TopologyAccessorInner {
    fn new(initial: NymRouteProvider) -> Self {
        TopologyAccessorInner {
            controlled_manually: AtomicBool::new(false),
            released_manual_control: Notify::new(),
            topology: RwLock::new(initial),
        }
    }

    async fn update(&self, new: Option<NymTopology>) {
        let mut guard = self.topology.write().await;

        match new {
            Some(updated) => {
                guard.update(updated);
            }
            None => guard.clear_topology(),
        }
    }
}

pub struct TopologyReadPermit<'a> {
    permit: RwLockReadGuard<'a, NymRouteProvider>,
}

impl Deref for TopologyReadPermit<'_> {
    type Target = NymRouteProvider;

    fn deref(&self) -> &Self::Target {
        &self.permit
    }
}

impl<'a> TopologyReadPermit<'a> {
    /// Using provided topology read permit, tries to get an immutable reference to the underlying
    /// topology. For obvious reasons the lifetime of the topology reference is bound to the permit.
    pub(crate) fn try_get_valid_topology_ref(
        &'a self,
        ack_recipient: &Recipient,
        packet_recipient: Option<&Recipient>,
    ) -> Result<&'a NymRouteProvider, NymTopologyError> {
        let route_provider = self.permit.deref();
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
}

impl<'a> From<RwLockReadGuard<'a, NymRouteProvider>> for TopologyReadPermit<'a> {
    fn from(permit: RwLockReadGuard<'a, NymRouteProvider>) -> Self {
        TopologyReadPermit { permit }
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

    pub async fn get_read_permit(&self) -> TopologyReadPermit<'_> {
        self.inner.topology.read().await.into()
    }

    pub(crate) async fn update_global_topology(&self, new_topology: Option<NymTopology>) {
        self.inner.update(new_topology).await;
    }

    pub(crate) async fn wait_for_released_manual_control(&self) {
        self.inner.released_manual_control.notified().await
    }

    #[deprecated(note = "use .current_route_provider instead")]
    pub async fn current_topology(&self) -> Option<NymTopology> {
        self.current_route_provider()
            .await
            .as_ref()
            .map(|p| p.topology.clone())
    }

    pub async fn current_route_provider(&self) -> Option<RwLockReadGuard<NymRouteProvider>> {
        let provider = self.inner.topology.read().await;
        if provider.topology.is_empty() {
            None
        } else {
            Some(provider)
        }
    }

    pub async fn manually_change_topology(&self, new_topology: NymTopology) {
        self.inner.controlled_manually.store(true, Ordering::SeqCst);
        self.inner.update(Some(new_topology)).await;
    }

    pub fn release_manual_control(&self) {
        self.inner
            .controlled_manually
            .store(false, Ordering::SeqCst);
        self.inner.released_manual_control.notify_waiters();
    }

    // only used by the client at startup to get a slightly more reasonable error message
    // (currently displays as unused because health checker is disabled due to required changes)
    pub async fn ensure_is_routable(&self) -> Result<(), NymTopologyError> {
        self.inner
            .topology
            .read()
            .await
            .topology
            .ensure_minimally_routable()
    }
}
