use crate::config::Config;
use crate::network::rest;
use crate::network::tendermint;
use crate::services::mixmining::health_check_runner;
use crypto::identity::MixIdentityKeyPair;
use healthcheck::HealthChecker;
use tokio::runtime::Runtime;

// allow for a generic validator
pub struct Validator {
    // when you re-introduce keys, check which ones you want:
    //    MixIdentityKeyPair (like 'nym-client' ) <- probably that one (after maybe renaming to just identity::KeyPair)
    //    encryption::KeyPair (like 'nym-mixnode' or 'sfw-provider')
    health_check_runner: health_check_runner::HealthCheckRunner,
    tendermint_abci: tendermint::Abci,
    rest_api: rest::Api,
}

// but for time being, since it's a dummy one, have it use dummy keys
impl Validator {
    pub fn new(config: Config) -> Self {
        let dummy_healthcheck_keypair = MixIdentityKeyPair::new();
        let hc = HealthChecker::new(
            config.get_mix_mining_resolution_timeout(),
            config.get_mix_mining_number_of_test_packets() as usize,
            dummy_healthcheck_keypair,
        );

        let health_check_runner = health_check_runner::HealthCheckRunner::new(
            config.get_mix_mining_directory_server(),
            config.get_mix_mining_run_delay(),
            hc,
        );

        let rest_api = rest::Api::new();

        Validator {
            health_check_runner,
            rest_api,

            // perhaps you might want to pass &config to the constructor
            // there to get the config.tendermint (assuming you create appropriate fields + getters)
            tendermint_abci: tendermint::Abci::new(),
        }
    }

    pub fn start(self) {
        let mut rt = Runtime::new().unwrap();
        rt.spawn(self.health_check_runner.run());
        rt.spawn(self.rest_api.run());
        rt.block_on(self.tendermint_abci.run());
    }
}
