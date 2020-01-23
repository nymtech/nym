use crate::built_info;
use log::{error, info, trace, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock as FRwLock;
use topology::NymTopology;

const NODE_HEALTH_THRESHOLD: f64 = 0.0;

// auxiliary type for ease of use
pub type TopologyInnerRef<T> = Arc<FRwLock<Inner<T>>>;

pub(crate) struct TopologyControl<T: NymTopology> {
    directory_server: String,
    refresh_rate: f64,
    inner: Arc<FRwLock<Inner<T>>>,
}

#[derive(Debug)]
enum TopologyError {
    HealthCheckError,
    NoValidPathsError,
}

impl<T: NymTopology> TopologyControl<T> {
    pub(crate) async fn new(directory_server: String, refresh_rate: f64) -> Self {
        let initial_topology = match Self::get_current_compatible_topology(directory_server.clone())
            .await
        {
            Ok(topology) => Some(topology),
            Err(err) => {
                error!("Initial topology is invalid - {:?}. Right now it will be impossible to send any packets through the mixnet!", err);
                None
            }
        };

        TopologyControl {
            directory_server,
            refresh_rate,
            inner: Arc::new(FRwLock::new(Inner::new(initial_topology))),
        }
    }

    async fn get_current_compatible_topology(directory_server: String) -> Result<T, TopologyError> {
        let full_topology = T::new(directory_server.clone());

        // run a healthcheck to determine healthy-ish nodes:
        // this is a temporary solution as the healthcheck will eventually be moved to validators
        let healthcheck_config = healthcheck::config::HealthCheck {
            directory_server,
            // those are literally irrelevant when running single check
            interval: 100000.0,
            resolution_timeout: 5.0,
            num_test_packets: 2,
        };
        let healthcheck = healthcheck::HealthChecker::new(healthcheck_config);
        let healthcheck_result = healthcheck.do_check().await;

        let healthcheck_scores = match healthcheck_result {
            Err(err) => {
                error!("Error while performing the healtcheck: {:?}", err);
                return Err(TopologyError::HealthCheckError);
            }
            Ok(scores) => scores,
        };

        let healthy_topology =
            healthcheck_scores.filter_topology_by_score(&full_topology, NODE_HEALTH_THRESHOLD);

        // for time being assume same versioning, i.e. if client is running X.Y.Z,
        // we're expecting mixes, providers and coconodes to also be running X.Y.Z
        let versioned_healthy_topology =
            healthy_topology.filter_node_versions("0.3.2", "0.3.2", built_info::PKG_VERSION);

        // make sure you can still send a packet through the network:
        if !versioned_healthy_topology.can_construct_path_through() {
            return Err(TopologyError::NoValidPathsError);
        }

        Ok(versioned_healthy_topology)
    }

    pub(crate) fn get_inner_ref(&self) -> Arc<FRwLock<Inner<T>>> {
        self.inner.clone()
    }

    async fn update_global_topology(&mut self, new_topology: Option<T>) {
        // acquire write lock
        let mut write_lock = self.inner.write().await;
        write_lock.topology = new_topology;
    }

    async fn should_update_topology(&mut self, new_topology: &Option<T>) -> bool {
        let read_lock = self.inner.read().await;
        match new_topology {
            // if new topology is invalid, we MUST update to it as it is impossible to send packets through
            None => true,
            Some(new_topology) => match &read_lock.topology {
                None => true,
                Some(old_topology) => new_topology != old_topology,
            },
        }
    }

    pub(crate) async fn run_refresher(mut self) {
        info!("Starting topology refresher");
        let delay_duration = Duration::from_secs_f64(self.refresh_rate);
        loop {
            trace!("Refreshing the topology");
            let new_topology_res =
                Self::get_current_compatible_topology(self.directory_server.clone()).await;

            let new_topology = match new_topology_res {
                Ok(topology) => Some(topology),
                Err(err) => {
                    warn!("the obtained topology seems to be invalid - {:?}, it will be impossible to send packets through", err);
                    None
                }
            };

            if self.should_update_topology(&new_topology).await {
                info!("Detected changes in topology - updating global view!");

                self.update_global_topology(new_topology).await;
            }

            tokio::time::delay_for(delay_duration).await;
        }
    }
}

pub struct Inner<T: NymTopology> {
    pub topology: Option<T>,
}

impl<T: NymTopology> Inner<T> {
    fn new(topology: Option<T>) -> Self {
        Inner { topology }
    }
}
