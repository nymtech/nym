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

use healthcheck::HealthChecker;
use log::*;
use std::time::Duration;
use topology::NymTopology;

pub struct HealthCheckRunner {
    directory_server: String,
    health_checker: HealthChecker,
    interval: Duration,
}

impl HealthCheckRunner {
    pub fn new(
        directory_server: String,
        interval: Duration,
        health_checker: HealthChecker,
    ) -> HealthCheckRunner {
        HealthCheckRunner {
            directory_server,
            health_checker,
            interval,
        }
    }

    pub async fn run(self) {
        println!("* starting periodic network healthcheck");
        debug!("healthcheck will run every {:?}", self.interval);
        loop {
            let full_topology =
                directory_client::presence::Topology::new(self.directory_server.clone());
            let version_filtered_topology =
                full_topology.filter_system_version(crate::built_info::PKG_VERSION);
            match self
                .health_checker
                .do_check(&version_filtered_topology)
                .await
            {
                Ok(health) => info!("current network health: \n{}", health),
                Err(err) => error!("failed to perform healthcheck - {:?}", err),
            };
            tokio::time::delay_for(self.interval).await;
        }
    }
}
