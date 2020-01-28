use crate::result::HealthCheckResult;
use crypto::identity::{MixnetIdentityKeyPair, MixnetIdentityPrivateKey, MixnetIdentityPublicKey};
use directory_client::requests::presence_topology_get::PresenceTopologyGetRequester;
use directory_client::DirectoryClient;
use log::{debug, error, info, trace};
use std::fmt::{Error, Formatter};
use std::marker::PhantomData;
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

pub struct HealthChecker<IDPair, Priv, Pub>
where
    IDPair: MixnetIdentityKeyPair<Priv, Pub>,
    Priv: MixnetIdentityPrivateKey,
    Pub: MixnetIdentityPublicKey,
{
    num_test_packets: usize,
    resolution_timeout: Duration,
    identity_keypair: IDPair,

    _phantom_private: PhantomData<Priv>,
    _phantom_public: PhantomData<Pub>,
}

impl<IDPair, Priv, Pub> HealthChecker<IDPair, Priv, Pub>
where
    IDPair: crypto::identity::MixnetIdentityKeyPair<Priv, Pub>,
    Priv: crypto::identity::MixnetIdentityPrivateKey,
    Pub: crypto::identity::MixnetIdentityPublicKey,
{
    pub fn new(
        resolution_timeout_f64: f64,
        num_test_packets: usize,
        identity_keypair: IDPair,
    ) -> Self {
        HealthChecker {
            resolution_timeout: Duration::from_secs_f64(resolution_timeout_f64),
            num_test_packets,
            identity_keypair,

            _phantom_private: PhantomData,
            _phantom_public: PhantomData,
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
