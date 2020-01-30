use crate::network::tendermint;
use crate::services::mixmining::health_check_runner;
use crypto::identity::MixIdentityKeyPair;
use healthcheck::HealthChecker;
use serde::Deserialize;
use tokio::runtime::Runtime;

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(rename(deserialize = "healthcheck"))]
    pub health_check: healthcheck::config::HealthCheck,
}

// allow for a generic validator
pub struct Validator {
    #[allow(dead_code)]
    identity_keypair: MixIdentityKeyPair,
    health_check_runner: health_check_runner::HealthCheckRunner,
    tendermint_abci: tendermint::Abci,
}

// but for time being, since it's a dummy one, have it use dummy keys
impl Validator {
    pub fn new(config: Config) -> Self {
        let dummy_keypair = MixIdentityKeyPair::new();
        let hc = HealthChecker::new(
            config.health_check.resolution_timeout,
            config.health_check.num_test_packets,
            dummy_keypair.clone(),
        );

        let health_check_runner = health_check_runner::HealthCheckRunner::new(
            config.health_check.directory_server.clone(),
            config.health_check.interval,
            hc,
        );

        Validator {
            identity_keypair: dummy_keypair,
            health_check_runner,
            tendermint_abci: tendermint::Abci::new(),
        }
    }

    pub fn start(self) {
        let mut rt = Runtime::new().unwrap();
        rt.spawn(self.health_check_runner.run());
        rt.block_on(self.tendermint_abci.run());
    }
}
