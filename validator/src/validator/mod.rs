use crate::validator::config::Config;
use crypto::identity::{DummyMixIdentityKeyPair, MixnetIdentityKeyPair};
use directory_client::presence::Topology;
use healthcheck::HealthChecker;
use log::{debug, error, info};
use std::time::Duration;
use tokio::runtime::Runtime;
use topology::NymTopology;

pub mod config;

// allow for a generic validator
pub struct Validator<IDPair: MixnetIdentityKeyPair> {
    config: Config,
    #[allow(dead_code)]
    identity_keypair: IDPair,
    heath_check: HealthChecker<IDPair>,
}

// but for time being, since it's a dummy one, have it use dummy keys
impl Validator<DummyMixIdentityKeyPair> {
    pub fn new(config: Config) -> Self {
        debug!("validator new");

        let dummy_keypair = DummyMixIdentityKeyPair::new();

        Validator {
            identity_keypair: dummy_keypair.clone(),
            heath_check: HealthChecker::new(
                config.health_check.resolution_timeout,
                config.health_check.num_test_packets,
                dummy_keypair,
            ),
            config,
        }
    }

    async fn healthcheck_runner<T: NymTopology>(&self) {
        let healthcheck_interval = Duration::from_secs_f64(self.config.health_check.interval);
        debug!("healthcheck will run every {:?}", healthcheck_interval);

        loop {
            let full_topology = T::new(self.config.health_check.directory_server.clone());
            let version_filtered_topology = full_topology.filter_node_versions(
                crate::built_info::PKG_VERSION,
                crate::built_info::PKG_VERSION,
                crate::built_info::PKG_VERSION,
            );

            match self.heath_check.do_check(&version_filtered_topology).await {
                Ok(health) => info!("current network health: \n{}", health),
                Err(err) => error!("failed to perform healthcheck - {:?}", err),
            };

            tokio::time::delay_for(healthcheck_interval).await;
        }
    }

    pub fn start(self) {
        debug!("validator run");

        let mut rt = Runtime::new().unwrap();

        let health_check_future = self.healthcheck_runner::<Topology>();
        rt.block_on(health_check_future);
    }
}
