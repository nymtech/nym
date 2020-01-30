use crate::result::HealthCheckResult;
use crypto::identity::MixnetIdentityKeyPair;
use log::trace;
use std::fmt::{Error, Formatter};
use std::time::Duration;
use topology::{NymTopology, NymTopologyError};

pub mod config;
mod path_check;
mod result;
mod score;

#[derive(Debug)]
pub enum HealthCheckerError {
    FailedToObtainTopologyError,
    InvalidTopologyError,
}

// required by std::error::Error
impl std::fmt::Display for HealthCheckerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        // just have implementation equivalent to derived debug
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for HealthCheckerError {}

impl From<topology::NymTopologyError> for HealthCheckerError {
    fn from(_: NymTopologyError) -> Self {
        use HealthCheckerError::*;
        InvalidTopologyError
    }
}

pub struct HealthChecker<IDPair: MixnetIdentityKeyPair> {
    num_test_packets: usize,
    resolution_timeout: Duration,
    identity_keypair: IDPair,
}

impl<IDPair: MixnetIdentityKeyPair> HealthChecker<IDPair> {
    pub fn new(
        resolution_timeout_f64: f64,
        num_test_packets: usize,
        identity_keypair: IDPair,
    ) -> Self {
        HealthChecker {
            resolution_timeout: Duration::from_secs_f64(resolution_timeout_f64),
            num_test_packets,
            identity_keypair,
        }
    }

    pub async fn do_check<T: NymTopology>(
        &self,
        current_topology: &T,
    ) -> Result<HealthCheckResult, HealthCheckerError> {
        trace!("going to perform a healthcheck!");

        let mut healthcheck_result = HealthCheckResult::calculate(
            current_topology,
            self.num_test_packets,
            self.resolution_timeout,
            &self.identity_keypair,
        )
        .await;
        healthcheck_result.sort_scores();
        Ok(healthcheck_result)
    }
}
