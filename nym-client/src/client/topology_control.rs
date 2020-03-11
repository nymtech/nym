use crate::built_info;
use crypto::identity::MixIdentityKeyPair;
use futures::lock::Mutex;
use healthcheck::HealthChecker;
use log::*;
use std::sync::Arc;
use std::time;
use std::time::Duration;
use tokio::runtime::Handle;
// use tokio::sync::RwLock;
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

    // Unless you absolutely need the entire topology, use `random_route` instead
    pub(crate) async fn get_current_topology_clone(&mut self) -> Option<T> {
        self.inner.lock().await.0.clone()
    }

    pub(crate) async fn get_all_clients(&mut self) -> Option<Vec<provider::Client>> {
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

    // this is a rather temporary solution as each client will have an associated provider
    // currently that is not implemented yet and there only exists one provider in the network
    pub(crate) async fn random_route(&mut self) -> Option<Vec<sphinx::route::Node>> {
        match &self.inner.lock().await.0 {
            None => None,
            Some(ref topology) => {
                let mut providers = topology.providers();
                if providers.is_empty() {
                    return None;
                }
                // unwrap is fine here as we asserted there is at least single provider
                let provider = providers.pop().unwrap().into();
                topology.route_to(provider).ok()
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
    number_test_packets: usize,
    node_score_threshold: f64,
}

impl TopologyRefresherConfig {
    pub(crate) fn new(
        directory_server: String,
        refresh_rate: time::Duration,
        identity_keypair: MixIdentityKeyPair,
        resolution_timeout: time::Duration,
        number_test_packets: usize,
        node_score_threshold: f64,
    ) -> Self {
        TopologyRefresherConfig {
            directory_server,
            refresh_rate,
            identity_keypair,
            resolution_timeout,
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
        let version_filtered_topology = full_topology.filter_node_versions(
            built_info::PKG_VERSION,
            built_info::PKG_VERSION,
            built_info::PKG_VERSION,
        );

        let healthcheck_result = self
            .health_checker
            .do_check(&version_filtered_topology)
            .await;
        let healthcheck_scores = match healthcheck_result {
            Err(err) => {
                error!("Error while performing the healthcheck: {:?}", err);
                return Err(TopologyError::HealthCheckError);
            }
            Ok(scores) => scores,
        };

        let healthy_topology = healthcheck_scores
            .filter_topology_by_score(&version_filtered_topology, self.node_score_threshold);

        // make sure you can still send a packet through the network:
        if !healthy_topology.can_construct_path_through() {
            return Err(TopologyError::NoValidPathsError);
        }

        Ok(healthy_topology)
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
                self.refresh().await;
                tokio::time::delay_for(self.refresh_rate).await;
            }
        })
    }
}
