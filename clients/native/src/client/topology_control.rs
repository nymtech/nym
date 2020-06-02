// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::built_info;
use log::*;
use nymsphinx::NodeAddressBytes;
use std::ops::Deref;
use std::sync::Arc;
use std::time;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::task::JoinHandle;
use topology::{provider, NymTopology};

// I'm extremely curious why compiler NEVER complained about lack of Debug here before
#[derive(Debug)]
pub(super) struct TopologyAccessorInner<T: NymTopology>(Option<T>);

impl<T: NymTopology> TopologyAccessorInner<T> {
    // `pub(super)` `deref` makes me feel better than `pub` `Deref` trait
    pub(super) fn deref(&self) -> &Option<T> {
        &self.0
    }
}

impl<T: NymTopology> TopologyAccessorInner<T> {
    fn new() -> Self {
        TopologyAccessorInner(None)
    }

    fn update(&mut self, new: Option<T>) {
        self.0 = new;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TopologyAccessor<T: NymTopology> {
    // `RwLock` *seems to* be the better approach for this as write access is only requested every
    // few seconds, while reads are needed every single packet generated.
    // However, proper benchmarks will be needed to determine if `RwLock` is indeed a better
    // approach than a `Mutex`
    inner: Arc<RwLock<TopologyAccessorInner<T>>>,
}

impl<T: NymTopology> TopologyAccessor<T> {
    pub(crate) fn new() -> Self {
        TopologyAccessor {
            inner: Arc::new(RwLock::new(TopologyAccessorInner::new())),
        }
    }

    // TODO: I really don't like having `TopologyAccessorInner` in return type,
    // but we can't return `T` because then we'd drop the read permit
    pub(super) async fn get_read_permit(&self) -> RwLockReadGuard<'_, TopologyAccessorInner<T>> {
        self.inner.read().await
    }

    async fn update_global_topology(&mut self, new_topology: Option<T>) {
        self.inner.write().await.update(new_topology);
    }

    pub(crate) async fn get_gateway_socket_url(&self, id: &str) -> Option<String> {
        match &self.inner.read().await.0 {
            None => None,
            Some(ref topology) => topology
                .gateways()
                .iter()
                .find(|gateway| gateway.pub_key == id)
                .map(|gateway| gateway.client_listener.clone()),
        }
    }

    // only used by the client at startup to get a slightly more reasonable error message
    // (currently displays as unused because healthchecker is disabled due to required changes)
    pub(crate) async fn is_routable(&self) -> bool {
        match &self.inner.read().await.0 {
            None => false,
            Some(ref topology) => topology.can_construct_path_through(),
        }
    }

    pub(crate) async fn get_all_clients(&self) -> Option<Vec<provider::Client>> {
        // TODO: this will need to be modified to instead return pairs (provider, client)
        match &self.inner.read().await.0 {
            None => None,
            Some(ref topology) => Some(
                topology
                    .providers()
                    .iter()
                    .flat_map(|provider| provider.registered_clients.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
        }
    }

    pub(crate) async fn random_route_to_gateway(
        &self,
        gateway_address: &NodeAddressBytes,
    ) -> Option<Vec<nymsphinx::Node>> {
        let guard = self.inner.read().await;
        let topology = guard.0.as_ref()?;
        topology.random_route_to_gateway(gateway_address).ok()
    }
}

pub(crate) struct TopologyRefresherConfig {
    directory_server: String,
    refresh_rate: time::Duration,
}

impl TopologyRefresherConfig {
    pub(crate) fn new(directory_server: String, refresh_rate: time::Duration) -> Self {
        TopologyRefresherConfig {
            directory_server,
            refresh_rate,
        }
    }
}

pub(crate) struct TopologyRefresher<T: NymTopology> {
    directory_server: String,
    topology_accessor: TopologyAccessor<T>,
    refresh_rate: Duration,
}

impl<T: 'static + NymTopology> TopologyRefresher<T> {
    pub(crate) fn new(
        cfg: TopologyRefresherConfig,
        topology_accessor: TopologyAccessor<T>,
    ) -> Self {
        TopologyRefresher {
            directory_server: cfg.directory_server,
            topology_accessor,
            refresh_rate: cfg.refresh_rate,
        }
    }

    async fn get_current_compatible_topology(&self) -> T {
        // note: this call makes it necessary that `T::new()`does *not* have 'static lifetime
        let full_topology = T::new(self.directory_server.clone()).await;
        // just filter by version and assume the validators will remove all bad behaving
        // nodes with the staking
        full_topology.filter_system_version(built_info::PKG_VERSION)
    }

    pub(crate) async fn refresh(&mut self) {
        trace!("Refreshing the topology");
        let new_topology = Some(self.get_current_compatible_topology().await);

        self.topology_accessor
            .update_global_topology(new_topology)
            .await;
    }

    pub(crate) async fn is_topology_routable(&self) -> bool {
        self.topology_accessor.is_routable().await
    }

    pub(crate) fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            loop {
                tokio::time::delay_for(self.refresh_rate).await;
                self.refresh().await;
            }
        })
    }
}
