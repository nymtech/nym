use crate::healthcheck::HealthChecker;

mod healthcheck;

fn main() {
    let health_checker = HealthChecker::new();
}
