use std::time::Duration;

use crypto::asymmetric::identity;

use crate::healthcheck::HealthChecker;

mod healthcheck;

fn main() {
    let resolution_timeout = Duration::from_millis(300);
    let connection_timeout = Duration::from_millis(300);
    let num_test_packets = 100;
    let identity_keypair = identity::KeyPair::new();
    let health_checker = HealthChecker::new(
        resolution_timeout,
        connection_timeout,
        num_test_packets,
        identity_keypair,
    );
}
