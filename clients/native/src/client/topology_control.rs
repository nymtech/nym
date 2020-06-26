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
use directory_client::DirectoryClient;
use log::*;
use nymsphinx::addressing::clients::Recipient;
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
struct TopologyAccessorInner<T: NymTopology>(Option<T>);

impl<T: NymTopology> Deref for TopologyAccessorInner<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
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

pub(super) struct TopologyReadPermit<'a, T: NymTopology> {
    permit: RwLockReadGuard<'a, TopologyAccessorInner<T>>,
}

impl<'a, T: NymTopology> TopologyReadPermit<'a, T> {
    /// Using provided topology read permit, tries to get an immutable reference to the underlying
    /// topology. For obvious reasons the lifetime of the topology reference is bound to the permit.
    pub(super) fn try_get_valid_topology_ref(
        &'a self,
        ack_recipient: &Recipient,
        packet_recipient: &Recipient,
    ) -> Option<&'a T> {
        // first we need to deref out of RwLockReadGuard
        // then we need to deref out of TopologyAccessorInner
        // then we must take ref of option, i.e. Option<&T>
        // and finally try to unwrap it to obtain &T
        let topology_ref_option = (*self.permit.deref()).as_ref();

        if topology_ref_option.is_none() {
            return None;
        }

        let topology_ref = topology_ref_option.unwrap();

        // see if it's possible to route the packet to both gateways
        if !topology_ref.can_construct_path_through()
            || !topology_ref.gateway_exists(&packet_recipient.gateway())
            || !topology_ref.gateway_exists(&ack_recipient.gateway())
        {
            None
        } else {
            Some(topology_ref)
        }
    }
}

impl<'a, T: NymTopology> From<RwLockReadGuard<'a, TopologyAccessorInner<T>>>
    for TopologyReadPermit<'a, T>
{
    fn from(read_permit: RwLockReadGuard<'a, TopologyAccessorInner<T>>) -> Self {
        TopologyReadPermit {
            permit: read_permit,
        }
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

    pub(super) async fn get_read_permit(&self) -> TopologyReadPermit<'_, T> {
        self.inner.read().await.into()
    }

    async fn update_global_topology(&mut self, new_topology: Option<T>) {
        self.inner.write().await.update(new_topology);
    }

    // pub(crate) async fn get_gateway_socket_url(&self, id: &str) -> Option<String> {
    //     match &self.inner.read().await.0 {
    //         None => None,
    //         Some(ref topology) => topology
    //             .gateways()
    //             .iter()
    //             .find(|gateway| gateway.identity_key == id)
    //             .map(|gateway| gateway.client_listener.clone()),
    //     }
    // }

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
    directory_client: directory_client::Client,
    topology_accessor: TopologyAccessor<T>,
    refresh_rate: Duration,
}

// TODO: consider (or maybe not) restoring generic TopologyRefresher<T>
impl TopologyRefresher<directory_client::Topology> {
    pub(crate) fn new_directory_client(
        cfg: TopologyRefresherConfig,
        topology_accessor: TopologyAccessor<directory_client::Topology>,
    ) -> Self {
        let directory_client_config = directory_client::Config::new(cfg.directory_server);
        let directory_client = directory_client::Client::new(directory_client_config);

        TopologyRefresher {
            directory_client,
            topology_accessor,
            refresh_rate: cfg.refresh_rate,
        }
    }

    async fn get_current_compatible_topology(&self) -> Option<directory_client::Topology> {
        match self.directory_client.get_topology().await {
            Err(err) => {
                error!("failed to get network topology! - {:?}", err);
                None
            }
            Ok(topology) => Some(topology.filter_system_version(built_info::PKG_VERSION)),
        }
    }

    pub(crate) async fn refresh(&mut self) {
        trace!("Refreshing the topology");
        let new_topology = self.get_current_compatible_topology().await;

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
