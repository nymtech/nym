use crate::validator::config::Config;
use crate::validator::health_check::HealthChecker;
use log::debug;

pub mod config;
mod health_check;

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
        self.heath_check.run()
    }
}
