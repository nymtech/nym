// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::result::HealthCheckResult;
use crypto::identity::MixIdentityKeyPair;
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

pub struct HealthChecker {
    num_test_packets: usize,
    resolution_timeout: Duration,
    connection_timeout: Duration,
    identity_keypair: MixIdentityKeyPair,
}

impl HealthChecker {
    pub fn new(
        resolution_timeout: Duration,
        connection_timeout: Duration,
        num_test_packets: usize,
        identity_keypair: MixIdentityKeyPair,
    ) -> Self {
        HealthChecker {
            resolution_timeout,
            connection_timeout,
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
            self.connection_timeout,
            &self.identity_keypair,
        )
        .await;
        healthcheck_result.sort_scores();
        Ok(healthcheck_result)
    }
}
