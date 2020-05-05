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
use crypto::identity::MixIdentityKeyPair;
use futures::lock::Mutex;
use healthcheck::HealthChecker;
use log::*;
use nymsphinx::DestinationAddressBytes;
use std::sync::Arc;
use std::time;
use std::time::Duration;
use tokio::runtime::Handle;
// use tokio::sync::RwLock;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use topology::{provider, NymTopology};

struct TopologyAccessorInner<T: NymTopology>(Option<T>);

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
    // TODO: this requires some actual benchmarking to determine if obtaining mutex is not going
    // to cause some bottlenecking and whether perhaps RwLock would be better
    inner: Arc<Mutex<TopologyAccessorInner<T>>>,
}

impl<T: NymTopology> TopologyAccessor<T> {
    pub(crate) fn new() -> Self {
        TopologyAccessor {
            inner: Arc::new(Mutex::new(TopologyAccessorInner::new())),
        }
    }

    async fn update_global_topology(&mut self, new_topology: Option<T>) {
        self.inner.lock().await.update(new_topology);
    }

    // not removed until healtchecker is not fully changed to use gateways instead of providers
    pub(crate) async fn get_provider_socket_addr(&self, id: &str) -> Option<SocketAddr> {
        match &self.inner.lock().await.0 {
            None => None,
            Some(ref topology) => topology
                .providers()
                .iter()
                .find(|provider| provider.pub_key == id)
                .map(|provider| provider.client_listener),
        }
    }

    pub(crate) async fn get_gateway_socket_url(&self, id: &str) -> Option<String> {
        match &self.inner.lock().await.0 {
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
        match &self.inner.lock().await.0 {
            None => false,
            Some(ref topology) => topology.can_construct_path_through(),
        }
    }

    // Unless you absolutely need the entire topology, use `random_route` instead
    pub(crate) async fn get_current_topology_clone(&self) -> Option<T> {
        self.inner.lock().await.0.clone()
    }

    pub(crate) async fn get_all_clients(&self) -> Option<Vec<provider::Client>> {
        // TODO: this will need to be modified to instead return pairs (provider, client)
        match &self.inner.lock().await.0 {
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

    pub(crate) async fn random_route_to_client(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Option<Vec<nymsphinx::Node>> {
        let b58_address = client_address.to_base58_string();
        let guard = self.inner.lock().await;
        let topology = guard.0.as_ref()?;

        let gateway = topology
            .gateways()
            .iter()
            .cloned()
            .find(|gateway| gateway.has_client(b58_address.clone()))?;

        topology.random_route_to(gateway.into()).ok()
    }

    // this is a rather temporary solution as each client will have an associated provider
    // currently that is not implemented yet and there only exists one provider in the network
    pub(crate) async fn random_route(&self) -> Option<Vec<nymsphinx::Node>> {
        match &self.inner.lock().await.0 {
            None => None,
            Some(ref topology) => {
                let mut gateways = topology.gateways();
                if gateways.is_empty() {
                    return None;
                }
                // unwrap is fine here as we asserted there is at least single provider
                let provider = gateways.pop().unwrap().into();
                topology.random_route_to(provider).ok()
            }
        }
    }
}

#[derive(Debug)]
enum TopologyError {
    HealthCheckError,
    NoValidPathsError,
}

pub(crate) struct TopologyRefresherConfig {
    directory_server: String,
    refresh_rate: time::Duration,
    identity_keypair: MixIdentityKeyPair,
    resolution_timeout: time::Duration,
    connection_timeout: time::Duration,
    number_test_packets: usize,
    node_score_threshold: f64,
}

impl TopologyRefresherConfig {
    pub(crate) fn new(
        directory_server: String,
        refresh_rate: time::Duration,
        identity_keypair: MixIdentityKeyPair,
        resolution_timeout: time::Duration,
        connection_timeout: time::Duration,
        number_test_packets: usize,
        node_score_threshold: f64,
    ) -> Self {
        TopologyRefresherConfig {
            directory_server,
            refresh_rate,
            identity_keypair,
            resolution_timeout,
            connection_timeout,
            number_test_packets,
            node_score_threshold,
        }
    }
}

pub(crate) struct TopologyRefresher<T: NymTopology> {
    directory_server: String,
    topology_accessor: TopologyAccessor<T>,
    health_checker: HealthChecker,
    refresh_rate: Duration,
    node_score_threshold: f64,
}

impl<T: 'static + NymTopology> TopologyRefresher<T> {
    pub(crate) fn new(
        cfg: TopologyRefresherConfig,
        topology_accessor: TopologyAccessor<T>,
    ) -> Self {
        // this is a temporary solution as the healthcheck will eventually be moved to validators
        let health_checker = healthcheck::HealthChecker::new(
            cfg.resolution_timeout,
            cfg.connection_timeout,
            cfg.number_test_packets,
            cfg.identity_keypair,
        );

        TopologyRefresher {
            directory_server: cfg.directory_server,
            topology_accessor,
            health_checker,
            refresh_rate: cfg.refresh_rate,
            node_score_threshold: cfg.node_score_threshold,
        }
    }

    async fn get_current_compatible_topology(&self) -> Result<T, TopologyError> {
        let full_topology = T::new(self.directory_server.clone());
        let version_filtered_topology =
            full_topology.filter_system_version(built_info::PKG_VERSION);

        // healthcheck needs some adjustments to work with gateways so for time being just dont run it
        return Ok(version_filtered_topology);

        //        let healthcheck_result = self
        //            .health_checker
        //            .do_check(&version_filtered_topology)
        //            .await;
        //        let healthcheck_scores = match healthcheck_result {
        //            Err(err) => {
        //                error!("Error while performing the healthcheck: {:?}", err);
        //                return Err(TopologyError::HealthCheckError);
        //            }
        //            Ok(scores) => scores,
        //        };
        //
        //        debug!("{}", healthcheck_scores);
        //
        //        let healthy_topology = healthcheck_scores
        //            .filter_topology_by_score(&version_filtered_topology, self.node_score_threshold);
        //
        //        // make sure you can still send a packet through the network:
        //        if !healthy_topology.can_construct_path_through() {
        //            return Err(TopologyError::NoValidPathsError);
        //        }
        //
        //        Ok(healthy_topology)
    }

    pub(crate) async fn refresh(&mut self) {
        trace!("Refreshing the topology");
        let new_topology = match self.get_current_compatible_topology().await {
            Ok(topology) => Some(topology),
            Err(err) => {
                warn!("the obtained topology seems to be invalid - {:?}, it will be impossible to send packets through", err);
                None
            }
        };

        self.topology_accessor
            .update_global_topology(new_topology)
            .await;
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
