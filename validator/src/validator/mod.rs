use crate::validator::config::Config;
use healthcheck::HealthChecker;
use log::debug;
use tokio::runtime::Runtime;

pub mod config;

pub struct Validator {
    heath_check: HealthChecker,
}

impl Validator {
    pub fn new(config: Config) -> Self {
        debug!("validator new");

        Validator {
            heath_check: HealthChecker::new(config.health_check),
        }
    }

    pub fn start(self) {
        debug!("validator run");

        let mut rt = Runtime::new().unwrap();

        let health_check_future = self.heath_check.run();

        let health_check_res = rt.block_on(health_check_future);
        assert!(health_check_res.is_ok()); // if it got here it means healthchecker failed anyway
    }
}
