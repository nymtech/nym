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

use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::params::DEFAULT_NUM_MIX_HOPS;
use std::ops::Deref;
use std::sync::Arc;
use std::time;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio::task::JoinHandle;
use topology::{nym_topology_from_bonds, NymTopology};

// I'm extremely curious why compiler NEVER complained about lack of Debug here before
#[derive(Debug)]
pub struct TopologyAccessorInner(Option<NymTopology>);

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

pub struct TopologyReadPermit<'a> {
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
        packet_recipient: Option<&Recipient>,
    ) -> Option<&'a NymTopology> {
        // Note: implicit deref with Deref for TopologyReadPermit is happening here
        let topology_ref_option = self.permit.as_ref();
        match topology_ref_option {
            None => None,
            Some(topology_ref) => {
                // see if it's possible to route the packet to both gateways
                if !topology_ref.can_construct_path_through(DEFAULT_NUM_MIX_HOPS)
                    || !topology_ref.gateway_exists(&ack_recipient.gateway())
                    || if let Some(packet_recipient) = packet_recipient {
                        !topology_ref.gateway_exists(&packet_recipient.gateway())
                    } else {
                        false
                    }
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
pub struct TopologyAccessor {
    // `RwLock` *seems to* be the better approach for this as write access is only requested every
    // few seconds, while reads are needed every single packet generated.
    // However, proper benchmarks will be needed to determine if `RwLock` is indeed a better
    // approach than a `Mutex`
    inner: Arc<RwLock<TopologyAccessorInner>>,
}

impl TopologyAccessor {
    pub fn new() -> Self {
        TopologyAccessor {
            inner: Arc::new(RwLock::new(TopologyAccessorInner::new())),
        }
    }

    pub async fn get_read_permit(&self) -> TopologyReadPermit<'_> {
        self.inner.read().await.into()
    }

    async fn update_global_topology(&mut self, new_topology: Option<NymTopology>) {
        self.inner.write().await.update(new_topology);
    }

    // only used by the client at startup to get a slightly more reasonable error message
    // (currently displays as unused because health checker is disabled due to required changes)
    pub async fn is_routable(&self) -> bool {
        match &self.inner.read().await.0 {
            None => false,
            Some(ref topology) => topology.can_construct_path_through(DEFAULT_NUM_MIX_HOPS),
        }
    }
}

impl Default for TopologyAccessor {
    fn default() -> Self {
        TopologyAccessor::new()
    }
}

pub struct TopologyRefresherConfig {
    available_validators: Vec<String>,
    mixnet_contract_address: String,
    refresh_rate: time::Duration,
}

impl TopologyRefresherConfig {
    pub fn new(
        available_validators: Vec<String>,
        mixnet_contract_address: String,
        refresh_rate: time::Duration,
    ) -> Self {
        TopologyRefresherConfig {
            available_validators,
            mixnet_contract_address,
            refresh_rate,
        }
    }
}

pub struct TopologyRefresher {
    validator_client: validator_client::Client,

    topology_accessor: TopologyAccessor,
    refresh_rate: Duration,

    was_latest_valid: bool,
}

impl TopologyRefresher {
    pub fn new(cfg: TopologyRefresherConfig, topology_accessor: TopologyAccessor) -> Self {
        let validator_client_config =
            validator_client::Config::new(cfg.available_validators, cfg.mixnet_contract_address);
        let validator_client = validator_client::Client::new(validator_client_config);

        TopologyRefresher {
            validator_client,
            topology_accessor,
            refresh_rate: cfg.refresh_rate,
            was_latest_valid: true,
        }
    }

    async fn get_current_compatible_topology(&mut self) -> Option<NymTopology> {
        // TODO: optimization for the future:
        // only refresh mixnodes on timer and refresh gateways only when
        // we have to send to a new, unknown, gateway

        let mixnodes = match self.validator_client.get_mix_nodes().await {
            Err(err) => {
                error!("failed to get network mixnodes - {}", err);
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self.validator_client.get_gateways().await {
            Err(err) => {
                error!("failed to get network gateways - {}", err);
                return None;
            }
            Ok(gateways) => gateways,
        };

        let topology = nym_topology_from_bonds(mixnodes, gateways);

        // TODO: I didn't want to change it now, but the expected system version should rather be put in config
        // rather than pulled from package version of `client_core`
        Some(topology.filter_system_version(env!("CARGO_PKG_VERSION")))
    }

    pub async fn refresh(&mut self) {
        trace!("Refreshing the topology");
        let new_topology = self.get_current_compatible_topology().await;

        if new_topology.is_none() && self.was_latest_valid {
            // if we failed to grab this topology, but the one before it was alright, let's assume
            // validator had a tiny hiccup and use the old data
            warn!("we're going to keep on using the old topology for this iteration");
            self.was_latest_valid = false;
            return;
        } else if new_topology.is_some() {
            self.was_latest_valid = true;
        }

        self.topology_accessor
            .update_global_topology(new_topology)
            .await;
    }

    pub async fn is_topology_routable(&self) -> bool {
        self.topology_accessor.is_routable().await
    }

    pub fn start(mut self, handle: &Handle) -> JoinHandle<()> {
        handle.spawn(async move {
            loop {
                tokio::time::sleep(self.refresh_rate).await;
                self.refresh().await;
            }
        })
    }
}
