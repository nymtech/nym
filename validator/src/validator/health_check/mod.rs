use crate::validator::config;
use log::debug;

pub(crate) struct HealthChecker {
    directory_server: String,
}

impl HealthChecker {
    pub fn new(config: config::HealthCheck) -> Self {
        HealthChecker {
            directory_server: config.directory_server,
        }
    }

    pub fn run(self) {
        debug!(
            "healthcheck run. will use directory at: {:?}",
            self.directory_server
        )
    }
}
