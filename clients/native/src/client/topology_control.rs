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
use nymsphinx::params::DEFAULT_NUM_MIX_HOPS;
use std::convert::TryInto;
use std::ops::Deref;
use std::sync::Arc;
use std::time;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::task::JoinHandle;
use topology::{gateway, NymTopology};

// I'm extremely curious why compiler NEVER complained about lack of Debug here before
#[derive(Debug)]
pub(super) struct TopologyAccessorInner(Option<NymTopology>);

impl AsRef<Option<NymTopology>> for TopologyAccessorInner {
    fn as_ref(&self) -> &Option<NymTopology> {
        &self.0
    }
}

impl TopologyAccessorInner {
    fn new() -> Self {
        TopologyAccessorInner(None)
    }

    fn update(&mut self, new: Option<NymTopology>) {
        self.0 = new;
    }
}

pub(super) struct TopologyReadPermit<'a> {
    permit: RwLockReadGuard<'a, TopologyAccessorInner>,
}

impl<'a> Deref for TopologyReadPermit<'a> {
    type Target = TopologyAccessorInner;

    fn deref(&self) -> &Self::Target {
        &self.permit
    }
}

impl<'a> TopologyReadPermit<'a> {
    /// Using provided topology read permit, tries to get an immutable reference to the underlying
    /// topology. For obvious reasons the lifetime of the topology reference is bound to the permit.
    pub(super) fn try_get_valid_topology_ref(
        &'a self,
        ack_recipient: &Recipient,
        packet_recipient: &Recipient,
    ) -> Option<&'a NymTopology> {
        // Note: implicit deref with Deref for TopologyReadPermit is happening here
        let topology_ref_option = self.permit.as_ref();
        match topology_ref_option {
            None => None,
            Some(topology_ref) => {
                // see if it's possible to route the packet to both gateways
                if !topology_ref.can_construct_path_through(DEFAULT_NUM_MIX_HOPS)
                    || !topology_ref.gateway_exists(&packet_recipient.gateway())
                    || !topology_ref.gateway_exists(&ack_recipient.gateway())
                {
                    None
                } else {
                    Some(topology_ref)
                }
            }
        }
    }
}

impl<'a> From<RwLockReadGuard<'a, TopologyAccessorInner>> for TopologyReadPermit<'a> {
    fn from(read_permit: RwLockReadGuard<'a, TopologyAccessorInner>) -> Self {
        TopologyReadPermit {
            permit: read_permit,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TopologyAccessor {
    // `RwLock` *seems to* be the better approach for this as write access is only requested every
    // few seconds, while reads are needed every single packet generated.
    // However, proper benchmarks will be needed to determine if `RwLock` is indeed a better
    // approach than a `Mutex`
    inner: Arc<RwLock<TopologyAccessorInner>>,
}

impl TopologyAccessor {
    pub(crate) fn new() -> Self {
        TopologyAccessor {
            inner: Arc::new(RwLock::new(TopologyAccessorInner::new())),
        }
    }

    pub(super) async fn get_read_permit(&self) -> TopologyReadPermit<'_> {
        self.inner.read().await.into()
    }

    async fn update_global_topology(&mut self, new_topology: Option<NymTopology>) {
        self.inner.write().await.update(new_topology);
    }

    // only used by the client at startup to get a slightly more reasonable error message
    // (currently displays as unused because healthchecker is disabled due to required changes)
    pub(crate) async fn is_routable(&self) -> bool {
        match &self.inner.read().await.0 {
            None => false,
            Some(ref topology) => topology.can_construct_path_through(DEFAULT_NUM_MIX_HOPS),
        }
    }

    pub(crate) async fn get_all_clients(&self) -> Option<Vec<gateway::Client>> {
        // TODO: this will need to be modified to instead return pairs (provider, client)
        match &self.inner.read().await.0 {
            None => None,
            Some(ref topology) => Some(
                topology
                    .gateways()
                    .iter()
                    .flat_map(|gateway| gateway.registered_clients.iter())
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

pub(crate) struct TopologyRefresher {
    directory_client: directory_client::Client,
    topology_accessor: TopologyAccessor,
    refresh_rate: Duration,
}

impl TopologyRefresher {
    pub(crate) fn new_directory_client(
        cfg: TopologyRefresherConfig,
        topology_accessor: TopologyAccessor,
    ) -> Self {
        let directory_client_config = directory_client::Config::new(cfg.directory_server);
        let directory_client = directory_client::Client::new(directory_client_config);

        TopologyRefresher {
            directory_client,
            topology_accessor,
            refresh_rate: cfg.refresh_rate,
        }
    }

    async fn get_current_compatible_topology(&self) -> Option<NymTopology> {
        match self.directory_client.get_topology().await {
            Err(err) => {
                error!("failed to get network topology! - {:?}", err);
                None
            }
            Ok(topology) => {
                let nym_topology: NymTopology = topology.try_into().ok()?;
                Some(nym_topology.filter_system_version(built_info::PKG_VERSION))
            }
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
