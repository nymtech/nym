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

use log::*;
use tokio::time::{self, Duration};

mod mixmining;
mod topology;
mod topology_active;
mod topology_removed;

pub async fn start(validator_base_url: &str) {
    let mut timer = time::interval(Duration::from_secs(10));
    loop {
        timer.tick().await;

        if let Err(err) = topology::renew_periodically(validator_base_url).await {
            warn!("Error refreshing topology: {}", err)
        };

        if let Err(err) = topology_active::renew_periodically(validator_base_url).await {
            warn!("Error refreshing active topology: {}", err)
        };

        if let Err(err) = topology_removed::renew_periodically(validator_base_url).await {
            warn!("Error refreshing removed topology: {}", err)
        };

        if let Err(err) = mixmining::renew_periodically(validator_base_url).await {
            warn!("Error refreshing mixmining report: {}", err)
        };
    }
}
