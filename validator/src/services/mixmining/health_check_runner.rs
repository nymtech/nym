use healthcheck::HealthChecker;
use log::*;
use std::time::Duration;
use topology::NymTopology;

pub struct HealthCheckRunner {
    directory_server: String,
    health_checker: HealthChecker,
    interval: Duration,
}

impl HealthCheckRunner {
    pub fn new(
        directory_server: String,
        interval: Duration,
        health_checker: HealthChecker,
    ) -> HealthCheckRunner {
        HealthCheckRunner {
            directory_server,
            health_checker,
            interval,
        }
    }

    pub async fn run(self) {
        println!("* starting periodic network healthcheck");
        debug!("healthcheck will run every {:?}", self.interval);
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
            tokio::time::delay_for(self.interval).await;
        }
    }
}
