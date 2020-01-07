use log::debug;

struct HealthChecker {}

impl HealthChecker {
    pub fn new() -> Self {
        HealthChecker {}
    }

    pub fn run(&self) {
        debug!("healthcheck run")
    }
}
