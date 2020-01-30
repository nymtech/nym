use healthcheck::HealthChecker;
use log::*;
use std::time::Duration;
use topology::NymTopology;

pub struct HealthCheckRunner {
    directory_server: String,
    health_checker: HealthChecker,
    interval: f64,
}

impl HealthCheckRunner {
    pub fn new(
        directory_server: String,
        interval: f64,
        health_checker: HealthChecker,
    ) -> HealthCheckRunner {
        HealthCheckRunner {
            directory_server,
            health_checker,
            interval,
        }
    }

    pub async fn run(self) {
        let healthcheck_interval = Duration::from_secs_f64(self.interval);
        debug!("healthcheck will run every {:?}", healthcheck_interval);
        loop {
            let full_topology =
                directory_client::presence::Topology::new(self.directory_server.clone());
            let version_filtered_topology = full_topology.filter_node_versions(
                crate::built_info::PKG_VERSION,
                crate::built_info::PKG_VERSION,
                crate::built_info::PKG_VERSION,
            );
            match self
                .health_checker
                .do_check(&version_filtered_topology)
                .await
            {
                Ok(health) => info!("current network health: \n{}", health),
                Err(err) => error!("failed to perform healthcheck - {:?}", err),
            };
            tokio::time::delay_for(healthcheck_interval).await;
        }
    }
}
