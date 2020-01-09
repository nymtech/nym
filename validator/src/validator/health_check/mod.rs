use crate::validator::config;
use directory_client::requests::presence_topology_get::PresenceTopologyGetRequester;
use directory_client::DirectoryClient;
use log::{debug, trace};
use std::time::Duration;

#[derive(Debug)]
pub enum HealthCheckerError {
    FailedToObtainTopologyError,
}

pub(crate) struct HealthChecker {
    directory_client: directory_client::Client,
    interval: Duration,
}

impl HealthChecker {
    pub fn new(config: config::HealthCheck) -> Self {
        debug!(
            "healthcheck will be using the following directory server: {:?}",
            config.directory_server
        );
        let directory_client_config = directory_client::Config::new(config.directory_server);
        HealthChecker {
            directory_client: directory_client::Client::new(directory_client_config),
            interval: Duration::from_secs_f64(config.interval),
        }
    }

    pub async fn run(self) -> Result<(), HealthCheckerError> {
        debug!("healthcheck will run every {:?}", self.interval,);

        loop {
            trace!("going to perform a healthcheck!");
            let current_topology = match self.directory_client.presence_topology.get() {
                Ok(topology) => topology,
                Err(_) => return Err(HealthCheckerError::FailedToObtainTopologyError),
            };

            trace!("current topology: {:?}", current_topology);

            tokio::time::delay_for(self.interval).await;
        }
    }
}
