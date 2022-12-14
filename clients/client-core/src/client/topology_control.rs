// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::spawn_future;
use futures::StreamExt;
use log::*;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::params::DEFAULT_NUM_MIX_HOPS;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, RwLockReadGuard};
use topology::{nym_topology_from_detailed, NymTopology, NymTopologyError};
use url::Url;

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
    ) -> Result<&'a NymTopology, NymTopologyError> {
        // 1. Have we managed to get anything from the refresher, i.e. have the nym-api queries gone through?
        let topology = self
            .permit
            .as_ref()
            .as_ref()
            .ok_or(NymTopologyError::EmptyNetworkTopology)?;

        // 2. does it have any mixnode at all?
        // 3. does it have any gateways at all?
        // 4. does it have a mixnode on each layer?
        topology.ensure_can_construct_path_through(DEFAULT_NUM_MIX_HOPS)?;

        // 5. does it contain OUR gateway (so that we could create an ack packet)?
        if !topology.gateway_exists(ack_recipient.gateway()) {
            return Err(NymTopologyError::NonExistentGatewayError {
                identity_key: ack_recipient.gateway().to_base58_string(),
            });
        }

        // 6. for our target recipient, does it contain THEIR gateway (so that we could create
        if let Some(recipient) = packet_recipient {
            if !topology.gateway_exists(recipient.gateway()) {
                return Err(NymTopologyError::NonExistentGatewayError {
                    identity_key: recipient.gateway().to_base58_string(),
                });
            }
        }

        Ok(topology)
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

    async fn update_global_topology(&self, new_topology: Option<NymTopology>) {
        self.inner.write().await.update(new_topology);
    }

    // only used by the client at startup to get a slightly more reasonable error message
    // (currently displays as unused because health checker is disabled due to required changes)
    pub async fn ensure_is_routable(&self) -> Result<(), NymTopologyError> {
        match &self.inner.read().await.0 {
            None => Err(NymTopologyError::EmptyNetworkTopology),
            Some(ref topology) => topology.ensure_can_construct_path_through(DEFAULT_NUM_MIX_HOPS),
        }
    }
}

impl Default for TopologyAccessor {
    fn default() -> Self {
        TopologyAccessor::new()
    }
}

pub struct TopologyRefresherConfig {
    nym_api_urls: Vec<Url>,
    refresh_rate: Duration,
    client_version: String,
}

impl TopologyRefresherConfig {
    pub fn new(nym_api_urls: Vec<Url>, refresh_rate: Duration, client_version: String) -> Self {
        TopologyRefresherConfig {
            nym_api_urls,
            refresh_rate,
            client_version,
        }
    }
}

pub struct TopologyRefresher {
    validator_client: validator_client::client::ApiClient,
    client_version: String,

    nym_api_urls: Vec<Url>,
    topology_accessor: TopologyAccessor,
    refresh_rate: Duration,

    currently_used_api: usize,
    was_latest_valid: bool,
}

impl TopologyRefresher {
    pub fn new(mut cfg: TopologyRefresherConfig, topology_accessor: TopologyAccessor) -> Self {
        cfg.nym_api_urls.shuffle(&mut thread_rng());

        TopologyRefresher {
            validator_client: validator_client::client::ApiClient::new(cfg.nym_api_urls[0].clone()),
            client_version: cfg.client_version,
            nym_api_urls: cfg.nym_api_urls,
            topology_accessor,
            refresh_rate: cfg.refresh_rate,
            currently_used_api: 0,
            was_latest_valid: true,
        }
    }

    fn use_next_nym_api(&mut self) {
        if self.nym_api_urls.len() == 1 {
            warn!("There's only a single nym API available - it won't be possible to use a different one");
            return;
        }

        self.currently_used_api = (self.currently_used_api + 1) % self.nym_api_urls.len();
        self.validator_client
            .change_nym_api(self.nym_api_urls[self.currently_used_api].clone())
    }

    /// Verifies whether nodes a reasonably distributed among all mix layers.
    ///
    /// In ideal world we would have 33% nodes on layer 1, 33% on layer 2 and 33% on layer 3.
    /// However, this is a rather unrealistic expectation, instead we check whether there exists
    /// a layer with more than 66% of nodes or with fewer than 15% and if so, we trigger a failure.
    ///
    /// # Arguments
    ///
    /// * `topology`: active topology constructed from validator api data
    fn check_layer_distribution(&self, active_topology: &NymTopology) -> bool {
        let mixes = active_topology.mixes();
        let mixnodes_count = active_topology.num_mixnodes();

        if active_topology.gateways().is_empty() {
            return false;
        }

        // trivial check to see if have at least a single node on each layer (regardless of active set size)
        if mixes.get(&1).is_none() || mixes.get(&2).is_none() || mixes.get(&3).is_none() {
            return false;
        }

        let upper_bound = (mixnodes_count as f32 * 0.66) as usize;
        let lower_bound = (mixnodes_count as f32 * 0.15) as usize;

        let layer1 = mixes.get(&1).unwrap().len();
        let layer2 = mixes.get(&2).unwrap().len();
        let layer3 = mixes.get(&3).unwrap().len();

        if layer1 < lower_bound || layer1 > upper_bound {
            warn!(
                "nodes: {}, layer1: {}, layer2: {}, layer3: {}",
                mixnodes_count, layer1, layer2, layer3
            );
            return false;
        }

        if layer2 < lower_bound || layer2 > upper_bound {
            warn!(
                "nodes: {}, layer1: {}, layer2: {}, layer3: {}",
                mixnodes_count, layer1, layer2, layer3
            );
            return false;
        }

        if layer3 < lower_bound || layer3 > upper_bound {
            warn!(
                "nodes: {}, layer1: {}, layer2: {}, layer3: {}",
                mixnodes_count, layer1, layer2, layer3
            );
            return false;
        }

        true
    }

    async fn get_current_compatible_topology(&self) -> Option<NymTopology> {
        // TODO: optimization for the future:
        // only refresh mixnodes on timer and refresh gateways only when
        // we have to send to a new, unknown, gateway

        let mixnodes = match self.validator_client.get_cached_active_mixnodes().await {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self.validator_client.get_cached_gateways().await {
            Err(err) => {
                error!("failed to get network gateways - {err}");
                return None;
            }
            Ok(gateways) => gateways,
        };

        let topology = nym_topology_from_detailed(mixnodes, gateways)
            .filter_system_version(&self.client_version);

        if !self.check_layer_distribution(&topology) {
            warn!("The current filtered active topology has extremely skewed layer distribution. It cannot be used.");
            None
        } else {
            Some(topology)
        }
    }

    pub async fn refresh(&mut self) {
        trace!("Refreshing the topology");
        let new_topology = self.get_current_compatible_topology().await;

        if new_topology.is_none() {
            self.use_next_nym_api();
        }

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

    pub async fn ensure_topology_is_routable(&self) -> Result<(), NymTopologyError> {
        self.topology_accessor.ensure_is_routable().await
    }

    pub fn start_with_shutdown(mut self, mut shutdown: task::ShutdownListener) {
        spawn_future(async move {
            debug!("Started TopologyRefresher with graceful shutdown support");

            #[cfg(not(target_arch = "wasm32"))]
            let mut interval = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
                self.refresh_rate,
            ));

            #[cfg(target_arch = "wasm32")]
            let mut interval =
                gloo_timers::future::IntervalStream::new(self.refresh_rate.as_millis() as u32);

            while !shutdown.is_shutdown() {
                tokio::select! {
                    _ = interval.next() => {
                        self.refresh().await;
                    },
                    _ = shutdown.recv() => {
                        log::trace!("TopologyRefresher: Received shutdown");
                    },
                }
            }
            shutdown.recv_timeout().await;
            log::debug!("TopologyRefresher: Exiting");
        })
    }
}
